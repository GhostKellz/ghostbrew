// SPDX-License-Identifier: GPL-2.0
/*
 * GhostBrew - sched-ext BPF scheduler for AMD Zen4/Zen5 X3D processors
 *
 * Copyright (C) 2025 ghostkellz <ckelley@ghostkellz.sh>
 *
 * Features:
 * - V-Cache CCD awareness for X3D chips
 * - BORE-inspired burst detection
 * - Zen topology-aware scheduling (CCD/CCX)
 * - Gaming process detection and prioritization
 * - Per-CCD dispatch queues for cache locality
 * - SMT awareness (prefer full-idle physical cores)
 * - Core compaction (consolidate gaming on V-Cache CCD)
 * - Kick preemption (preempt batch tasks for gaming)
 */

#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>
#include "scx/common.bpf.h"

char _license[] SEC("license") = "GPL";

/*
 * Constants
 */
#define GHOSTBREW_VERSION	"0.1.0"
#define MAX_CPUS		256
#define MAX_CCDS		8
#define NSEC_PER_MSEC		1000000ULL

/* DSQ IDs: 0 = fallback shared, 1-8 = per-CCD */
#define FALLBACK_DSQ		0
#define CCD_DSQ_BASE		1

/* Priority classes for preemption */
#define PRIO_GAMING		0
#define PRIO_INTERACTIVE	1
#define PRIO_BATCH		2

/*
 * Tunables (set from userspace via .rodata)
 */
const volatile u32 nr_cpus_possible = 64;
const volatile u32 nr_ccds = 2;
const volatile u32 vcache_ccd = 0;
const volatile bool gaming_mode = true;
const volatile bool smt_enabled = true;
const volatile u64 burst_threshold_ns = 2 * NSEC_PER_MSEC;
const volatile u64 slice_ns = 3 * NSEC_PER_MSEC;
const volatile bool debug_mode = false;

/*
 * User-exit info for error reporting
 */
UEI_DEFINE(uei);

/*
 * Statistics (exported to userspace)
 */
u64 nr_enqueued = 0;
u64 nr_dispatched = 0;
u64 nr_direct_dispatched = 0;
u64 nr_gaming_tasks = 0;
u64 nr_interactive_tasks = 0;
u64 nr_vcache_migrations = 0;
u64 nr_ccd_local = 0;
u64 nr_ccd_cross = 0;
u64 nr_smt_idle_picks = 0;
u64 nr_compaction_overflows = 0;
u64 nr_preempt_kicks = 0;
/* Phase 4a statistics */
u64 nr_proton_tasks = 0;
u64 nr_parent_chain_detects = 0;
u64 nr_userspace_hint_detects = 0;
u64 nr_prefcore_placements = 0;
/* Phase 4b statistics */
u64 nr_gpu_feeder_tasks = 0;
/* Phase 4c statistics */
u64 nr_vm_vcpu_tasks = 0;
u64 nr_gaming_vm_vcpus = 0;
u64 nr_dev_vm_vcpus = 0;
u64 nr_container_tasks = 0;
u64 nr_ai_container_tasks = 0;
/* Phase 4d statistics */
u64 nr_cgroup_classifications = 0;
u64 nr_cgroup_gaming = 0;

/*
 * Per-CPU context - populated from userspace
 */
struct cpu_ctx {
	u32 ccd;
	u32 ccx;
	u32 node;
	s32 smt_sibling;  /* SMT sibling CPU, -1 if none */
	bool is_vcache;
	u8 _pad[3];
};

struct {
	__uint(type, BPF_MAP_TYPE_ARRAY);
	__uint(max_entries, MAX_CPUS);
	__type(key, u32);
	__type(value, struct cpu_ctx);
} cpu_ctxs SEC(".maps");

/*
 * Gaming PIDs - populated by userspace via /proc scanning
 * Key: PID, Value: workload class (1 = gaming, 4 = AI, etc.)
 */
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 1024);
	__type(key, u32);
	__type(value, u32);
} gaming_pids SEC(".maps");

/*
 * CPU prefcore rankings - populated by userspace from amd_pstate
 * Higher values = AMD prefers this core for boosting
 */
struct {
	__uint(type, BPF_MAP_TYPE_ARRAY);
	__uint(max_entries, MAX_CPUS);
	__type(key, u32);
	__type(value, u32);
} prefcore_rankings SEC(".maps");

/*
 * VM vCPU PIDs - populated by userspace from QEMU/KVM detection
 * Key: PID, Value: workload class (WORKLOAD_VM_DEV, WORKLOAD_VM_GAMING, etc.)
 */
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 512);
	__type(key, u32);
	__type(value, u32);
} vm_vcpu_pids SEC(".maps");

/*
 * Container PIDs - populated by userspace from container runtime detection
 * Key: PID, Value: workload class (WORKLOAD_CONTAINER, WORKLOAD_AI, etc.)
 */
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 4096);
	__type(key, u32);
	__type(value, u32);
} container_pids SEC(".maps");

/*
 * Cgroup classification - populated by userspace from cgroup path analysis
 * Key: cgroup ID (u64), Value: workload class
 * Allows classification by systemd slice (gaming.slice), docker cgroups, etc.
 */
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 8192);
	__type(key, u64);
	__type(value, u32);
} cgroup_classes SEC(".maps");

/*
 * Per-CCD load tracking for core compaction
 */
struct ccd_load {
	u64 nr_gaming;   /* Number of gaming tasks on this CCD */
	u64 nr_tasks;    /* Total running tasks on this CCD */
};

struct {
	__uint(type, BPF_MAP_TYPE_ARRAY);
	__uint(max_entries, MAX_CCDS);
	__type(key, u32);
	__type(value, struct ccd_load);
} ccd_loads SEC(".maps");

/*
 * Per-CPU running state for kick preemption
 */
struct cpu_run_state {
	u32 priority_class;  /* PRIO_GAMING, PRIO_INTERACTIVE, or PRIO_BATCH */
	u32 pid;             /* Running task PID */
	u64 started_at;      /* When task started running */
};

struct {
	__uint(type, BPF_MAP_TYPE_ARRAY);
	__uint(max_entries, MAX_CPUS);
	__type(key, u32);
	__type(value, struct cpu_run_state);
} cpu_run_states SEC(".maps");

/*
 * Workload classification types
 */
#define WORKLOAD_UNKNOWN	0
#define WORKLOAD_GAMING		1
#define WORKLOAD_INTERACTIVE	2
#define WORKLOAD_BATCH		3
#define WORKLOAD_AI		4
#define WORKLOAD_VM_DEV		5	/* Development VM vCPU */
#define WORKLOAD_VM_GAMING	6	/* Gaming VM vCPU */
#define WORKLOAD_CONTAINER	7	/* Container process */

/*
 * Per-task context for burst tracking and classification
 */
struct task_ctx {
	u64 burst_time;
	u64 last_run_at;
	u64 classification_time;	/* When was classification done */
	u32 preferred_ccd;
	u32 last_ccd;
	u32 workload_class;		/* WORKLOAD_* type */
	bool is_gaming;
	bool is_interactive;
	bool is_proton;			/* Specifically Wine/Proton task */
	bool is_gpu_feeder;		/* GPU-feeding thread (Vulkan/OpenGL) */
	bool wants_vcache;
	bool classification_valid;	/* Has been classified */
};

struct {
	__uint(type, BPF_MAP_TYPE_TASK_STORAGE);
	__uint(map_flags, BPF_F_NO_PREALLOC);
	__type(key, int);
	__type(value, struct task_ctx);
} task_ctxs SEC(".maps");

/*
 * Helper: Get or create task context
 */
static struct task_ctx *get_task_ctx(struct task_struct *p)
{
	return bpf_task_storage_get(&task_ctxs, p, NULL,
				    BPF_LOCAL_STORAGE_GET_F_CREATE);
}

/*
 * Helper: Get CPU context
 */
static struct cpu_ctx *get_cpu_ctx(s32 cpu)
{
	u32 key = cpu;
	return bpf_map_lookup_elem(&cpu_ctxs, &key);
}

/*
 * Helper: Get DSQ ID for a CCD
 */
static u64 ccd_to_dsq(u32 ccd)
{
	if (ccd >= MAX_CCDS)
		return FALLBACK_DSQ;
	return CCD_DSQ_BASE + ccd;
}

/*
 * Helper: Check comm for gaming patterns
 * Returns: 0 = not gaming, 1 = gaming (generic), 2 = proton/wine specifically
 */
static int check_gaming_comm(const char *comm)
{
	/* Wine/Proton patterns - return 2 for Proton-specific */
	if (comm[0] == 'w' && comm[1] == 'i' && comm[2] == 'n' && comm[3] == 'e')
		return 2;  /* wine, wine64, wineserver, wine-preloader */
	if (comm[0] == 'p' && comm[1] == 'r' && comm[2] == 'o' && comm[3] == 't')
		return 2;  /* proton */

	/* Steam/Proton container processes */
	if (comm[0] == 's' && comm[1] == 't' && comm[2] == 'e' && comm[3] == 'a' && comm[4] == 'm')
		return 1;  /* steam */
	if (comm[0] == 'b' && comm[1] == 'w' && comm[2] == 'r' && comm[3] == 'a' && comm[4] == 'p')
		return 1;  /* bwrap (bubblewrap) */
	if (comm[0] == 'p' && comm[1] == 'r' && comm[2] == 'e' && comm[3] == 's' && comm[4] == 's')
		return 1;  /* pressure-vessel */
	if (comm[0] == 'g' && comm[1] == 'a' && comm[2] == 'm' && comm[3] == 'e')
		return 1;  /* game*, gamescope */

	/* Game launchers */
	if (comm[0] == 'l' && comm[1] == 'u' && comm[2] == 't' && comm[3] == 'r' && comm[4] == 'i')
		return 1;  /* lutris */
	if (comm[0] == 'h' && comm[1] == 'e' && comm[2] == 'r' && comm[3] == 'o' && comm[4] == 'i')
		return 1;  /* heroic */

	/* .exe processes (common Wine pattern) */
	int len = 0;
	#pragma unroll
	for (int i = 0; i < TASK_COMM_LEN && comm[i]; i++)
		len = i + 1;
	if (len >= 4 && comm[len-4] == '.' && comm[len-3] == 'e' &&
	    comm[len-2] == 'x' && comm[len-1] == 'e')
		return 2;  /* .exe = Wine/Proton */

	return 0;
}

/*
 * Helper: Check if comm indicates a GPU-feeding thread (Vulkan/OpenGL)
 * These threads feed work to the GPU and benefit from low latency
 */
static bool check_gpu_feeder_comm(const char *comm)
{
	/* Vulkan threads */
	if (comm[0] == 'v' && comm[1] == 'k')
		return true;  /* vk*, VkThread */
	if (comm[0] == 'V' && comm[1] == 'k')
		return true;
	if (comm[0] == 'v' && comm[1] == 'u' && comm[2] == 'l' && comm[3] == 'k')
		return true;  /* vulkan* */

	/* OpenGL threads */
	if (comm[0] == 'g' && comm[1] == 'l')
		return true;  /* gl*, GLThread */
	if (comm[0] == 'G' && comm[1] == 'L')
		return true;
	if (comm[0] == 'o' && comm[1] == 'p' && comm[2] == 'e' && comm[3] == 'n' && comm[4] == 'g')
		return true;  /* opengl* */

	/* DXVK/VKD3D (Wine translation layers) */
	if (comm[0] == 'd' && comm[1] == 'x' && comm[2] == 'v' && comm[3] == 'k')
		return true;  /* dxvk* */
	if (comm[0] == 'v' && comm[1] == 'k' && comm[2] == 'd' && comm[3] == '3' && comm[4] == 'd')
		return true;  /* vkd3d* */

	/* NVIDIA driver threads */
	if (comm[0] == 'n' && comm[1] == 'v' && (comm[2] == '_' || comm[2] == 'i'))
		return true;  /* nv_*, nvidia* */
	if (comm[0] == 't' && comm[1] == 'h' && comm[2] == 'r' && comm[3] == 'e' &&
	    comm[4] == 'a' && comm[5] == 'd' && comm[6] == 'e' && comm[7] == 'd')
		return true;  /* threaded_gl */

	return false;
}

/*
 * Helper: Check if PID is marked as gaming by userspace
 */
static u32 check_userspace_gaming_hint(struct task_struct *p)
{
	u32 pid = BPF_CORE_READ(p, tgid);
	u32 *class = bpf_map_lookup_elem(&gaming_pids, &pid);
	if (class)
		return *class;
	return 0;
}

/*
 * Helper: Check if PID is a VM vCPU thread
 * Returns workload class (WORKLOAD_VM_DEV, WORKLOAD_VM_GAMING, etc.) or 0
 */
static u32 check_vm_vcpu_hint(struct task_struct *p)
{
	u32 pid = BPF_CORE_READ(p, pid);  /* Use thread ID for vCPUs */
	u32 *class = bpf_map_lookup_elem(&vm_vcpu_pids, &pid);
	if (class) {
		__sync_fetch_and_add(&nr_vm_vcpu_tasks, 1);
		if (*class == WORKLOAD_VM_GAMING)
			__sync_fetch_and_add(&nr_gaming_vm_vcpus, 1);
		else if (*class == WORKLOAD_VM_DEV)
			__sync_fetch_and_add(&nr_dev_vm_vcpus, 1);
		return *class;
	}
	return 0;
}

/*
 * Helper: Check if PID is a container process
 * Returns workload class (WORKLOAD_CONTAINER, WORKLOAD_AI, etc.) or 0
 */
static u32 check_container_hint(struct task_struct *p)
{
	u32 pid = BPF_CORE_READ(p, tgid);
	u32 *class = bpf_map_lookup_elem(&container_pids, &pid);
	if (class) {
		__sync_fetch_and_add(&nr_container_tasks, 1);
		if (*class == WORKLOAD_AI)
			__sync_fetch_and_add(&nr_ai_container_tasks, 1);
		return *class;
	}
	return 0;
}

/*
 * Helper: Check cgroup classification
 * Looks up task's cgroup ID in the cgroup_classes map populated by userspace
 * Returns workload class or 0 if not classified
 */
static u32 check_cgroup_class(struct task_struct *p)
{
	struct cgroup *cgrp;
	u64 cgid;
	u32 *class;

	/* Get the task's default cgroup */
	cgrp = BPF_CORE_READ(p, cgroups, dfl_cgrp);
	if (!cgrp)
		return 0;

	/* Get cgroup ID from kernfs node */
	cgid = BPF_CORE_READ(cgrp, kn, id);
	if (!cgid)
		return 0;

	/* Look up classification */
	class = bpf_map_lookup_elem(&cgroup_classes, &cgid);
	if (class) {
		__sync_fetch_and_add(&nr_cgroup_classifications, 1);
		if (*class == WORKLOAD_GAMING)
			__sync_fetch_and_add(&nr_cgroup_gaming, 1);
		return *class;
	}

	return 0;
}

/*
 * Helper: Walk parent chain looking for gaming indicators
 * Returns: 0 = not gaming, 1 = gaming, 2 = proton
 */
static int check_parent_chain_gaming(struct task_struct *p)
{
	struct task_struct *parent;
	char comm[TASK_COMM_LEN];
	int result;

	parent = BPF_CORE_READ(p, real_parent);

	/* Check up to 5 levels of parents */
	#pragma unroll
	for (int i = 0; i < 5 && parent; i++) {
		BPF_CORE_READ_STR_INTO(&comm, parent, comm);

		result = check_gaming_comm(comm);
		if (result > 0) {
			__sync_fetch_and_add(&nr_parent_chain_detects, 1);
			return result;
		}

		/* Move to grandparent */
		struct task_struct *grandparent = BPF_CORE_READ(parent, real_parent);
		if (grandparent == parent)
			break;  /* Reached init */
		parent = grandparent;
	}

	return 0;
}

/*
 * Helper: Comprehensive gaming task detection with caching
 * Also detects GPU-feeding threads (Vulkan/OpenGL) for latency optimization
 */
static bool is_gaming_task(struct task_struct *p)
{
	struct task_ctx *tctx;
	char comm[TASK_COMM_LEN];
	int gaming_type;
	u32 hint;
	bool gpu_feeder = false;

	/* Check cache first */
	tctx = get_task_ctx(p);
	if (tctx && tctx->classification_valid)
		return tctx->is_gaming;

	/* Level 1: Direct comm check (fastest) */
	BPF_CORE_READ_STR_INTO(&comm, p, comm);
	gaming_type = check_gaming_comm(comm);

	/* Check for GPU-feeding threads (Vulkan/OpenGL) */
	if (check_gpu_feeder_comm(comm)) {
		gpu_feeder = true;
		__sync_fetch_and_add(&nr_gpu_feeder_tasks, 1);
		/* GPU feeders are treated as gaming for scheduling purposes */
		if (gaming_type == 0)
			gaming_type = 1;
	}

	if (gaming_type > 0)
		goto found_gaming;

	/* Level 2: Userspace hint check (gaming_pids map) */
	hint = check_userspace_gaming_hint(p);
	if (hint == WORKLOAD_GAMING) {
		__sync_fetch_and_add(&nr_userspace_hint_detects, 1);
		gaming_type = 1;
		goto found_gaming;
	}

	/* Level 3: Cgroup classification (gaming.slice, docker, etc.) */
	hint = check_cgroup_class(p);
	if (hint == WORKLOAD_GAMING) {
		gaming_type = 1;
		goto found_gaming;
	}

	/* Level 4: Parent chain check (slower, but catches child processes) */
	gaming_type = check_parent_chain_gaming(p);
	if (gaming_type > 0)
		goto found_gaming;

	/* Level 5: Check for VM vCPU - gaming VMs get gaming treatment */
	hint = check_vm_vcpu_hint(p);
	if (hint == WORKLOAD_VM_GAMING) {
		gaming_type = 1;
		goto found_gaming;
	}

	/* Not a gaming task - check if VM/container/cgroup for proper classification */
	if (tctx) {
		tctx->is_gaming = false;
		tctx->is_proton = false;
		tctx->is_gpu_feeder = false;

		/* Check for VM or container workload */
		if (hint > 0 && hint != WORKLOAD_GAMING) {
			/* VM vCPU (dev or AI) or cgroup classification */
			tctx->workload_class = hint;
		} else {
			u32 container_class = check_container_hint(p);
			if (container_class > 0) {
				tctx->workload_class = container_class;
			} else {
				/* Final fallback: check cgroup for non-gaming classes */
				u32 cgroup_class = check_cgroup_class(p);
				if (cgroup_class > 0 && cgroup_class != WORKLOAD_GAMING) {
					tctx->workload_class = cgroup_class;
				} else {
					tctx->workload_class = WORKLOAD_BATCH;
				}
			}
		}

		tctx->classification_valid = true;
		tctx->classification_time = bpf_ktime_get_ns();
	}
	return false;

found_gaming:
	if (tctx) {
		tctx->is_gaming = true;
		tctx->is_proton = (gaming_type == 2);
		tctx->is_gpu_feeder = gpu_feeder;
		tctx->workload_class = WORKLOAD_GAMING;
		tctx->classification_valid = true;
		tctx->classification_time = bpf_ktime_get_ns();
	}
	if (gaming_type == 2)
		__sync_fetch_and_add(&nr_proton_tasks, 1);
	return true;
}

/*
 * Helper: Get prefcore ranking for a CPU (0 if not set)
 */
static u32 get_prefcore_ranking(s32 cpu)
{
	u32 key = cpu;
	u32 *ranking = bpf_map_lookup_elem(&prefcore_rankings, &key);
	return ranking ? *ranking : 0;
}

/*
 * Helper: Pick idle CPU from a specific CCD with SMT awareness
 *
 * When prefer_smt_idle is true, we prefer CPUs where the entire physical
 * core is idle (both SMT siblings idle). This avoids contention for shared
 * core resources in latency-sensitive gaming workloads.
 *
 * When prefer_prefcore is true, we prefer CPUs with higher prefcore ranking
 * (AMD's preferred cores for boosting).
 */
static s32 pick_idle_cpu_in_ccd(struct task_struct *p, u32 target_ccd, bool prefer_smt_idle)
{
	const struct cpumask *idle_smtmask = NULL;
	struct cpu_ctx *cctx;
	s32 cpu, best_cpu = -1;
	u32 best_ranking = 0;

	if (target_ccd >= nr_ccds)
		return -1;

	/* Get SMT-idle mask if we prefer full-idle cores */
	if (prefer_smt_idle && smt_enabled)
		idle_smtmask = scx_bpf_get_idle_smtmask();

	/*
	 * First pass: find SMT-idle CPUs, preferring highest prefcore ranking
	 */
	if (idle_smtmask) {
		bpf_for(cpu, 0, nr_cpus_possible) {
			if (cpu >= MAX_CPUS)
				break;

			cctx = get_cpu_ctx(cpu);
			if (!cctx || cctx->ccd != target_ccd)
				continue;

			if (!bpf_cpumask_test_cpu(cpu, p->cpus_ptr))
				continue;

			/* Check if entire physical core is idle */
			if (bpf_cpumask_test_cpu(cpu, idle_smtmask)) {
				u32 ranking = get_prefcore_ranking(cpu);
				/*
				 * Track best candidate by prefcore ranking.
				 * We continue scanning to find the highest-ranked idle core.
				 */
				if (best_cpu < 0 || ranking > best_ranking) {
					best_cpu = cpu;
					best_ranking = ranking;
				}
			}
		}

		/* Try to claim the best SMT-idle CPU we found */
		if (best_cpu >= 0 && scx_bpf_test_and_clear_cpu_idle(best_cpu)) {
			scx_bpf_put_idle_cpumask(idle_smtmask);
			__sync_fetch_and_add(&nr_smt_idle_picks, 1);
			if (best_ranking > 0)
				__sync_fetch_and_add(&nr_prefcore_placements, 1);
			return best_cpu;
		}

		scx_bpf_put_idle_cpumask(idle_smtmask);
	}

	/*
	 * Second pass: find any idle CPU in the CCD, preferring high prefcore ranking
	 */
	best_cpu = -1;
	best_ranking = 0;

	bpf_for(cpu, 0, nr_cpus_possible) {
		if (cpu >= MAX_CPUS)
			break;

		cctx = get_cpu_ctx(cpu);
		if (!cctx || cctx->ccd != target_ccd)
			continue;

		if (!bpf_cpumask_test_cpu(cpu, p->cpus_ptr))
			continue;

		u32 ranking = get_prefcore_ranking(cpu);
		if (best_cpu < 0 || ranking > best_ranking) {
			best_cpu = cpu;
			best_ranking = ranking;
		}
	}

	/* Try to claim the best CPU we found */
	if (best_cpu >= 0 && scx_bpf_test_and_clear_cpu_idle(best_cpu)) {
		if (best_ranking > 0)
			__sync_fetch_and_add(&nr_prefcore_placements, 1);
		return best_cpu;
	}

	return -1;
}

/*
 * Helper: Get CCD load
 */
static struct ccd_load *get_ccd_load(u32 ccd)
{
	return bpf_map_lookup_elem(&ccd_loads, &ccd);
}

/*
 * Helper: Find a CPU to kick in target CCD for preemption
 *
 * Returns CPU running the lowest priority task that can be preempted.
 */
static s32 find_kick_victim_in_ccd(u32 target_ccd, u32 min_priority)
{
	struct cpu_ctx *cctx;
	struct cpu_run_state *state;
	s32 victim_cpu = -1;
	u32 worst_priority = 0;
	s32 cpu;

	bpf_for(cpu, 0, nr_cpus_possible) {
		if (cpu >= MAX_CPUS)
			break;

		cctx = get_cpu_ctx(cpu);
		if (!cctx || cctx->ccd != target_ccd)
			continue;

		u32 key = cpu;
		state = bpf_map_lookup_elem(&cpu_run_states, &key);
		if (!state)
			continue;

		/* Find CPU running lowest priority task that can be preempted */
		if (state->priority_class > min_priority &&
		    state->priority_class >= worst_priority) {
			worst_priority = state->priority_class;
			victim_cpu = cpu;
		}
	}

	return victim_cpu;
}

/*
 * ops.select_cpu - Select CPU for task with V-Cache awareness
 *
 * Strategy:
 * 1. Gaming/interactive tasks -> prefer V-Cache CCD with SMT-idle cores
 * 2. Batch tasks when gaming present -> overflow to non-V-Cache CCDs (compaction)
 * 3. Fallback to any idle CPU
 */
s32 BPF_STRUCT_OPS(ghostbrew_select_cpu, struct task_struct *p,
		   s32 prev_cpu, u64 wake_flags)
{
	struct task_ctx *tctx;
	struct cpu_ctx *prev_cctx;
	struct ccd_load *vcache_load;
	bool is_idle = false;
	bool vcache_has_gaming = false;
	s32 cpu = -1;
	u32 target_ccd;

	tctx = get_task_ctx(p);
	if (!tctx)
		return prev_cpu;

	prev_cctx = get_cpu_ctx(prev_cpu);
	if (!prev_cctx)
		return prev_cpu;

	/* Update task classification */
	tctx->is_gaming = gaming_mode && is_gaming_task(p);
	tctx->is_interactive = tctx->burst_time < burst_threshold_ns;
	tctx->wants_vcache = tctx->is_gaming || (tctx->is_interactive && gaming_mode);

	/* Check if V-Cache CCD has gaming tasks (for compaction decisions) */
	vcache_load = get_ccd_load(vcache_ccd);
	if (vcache_load)
		vcache_has_gaming = vcache_load->nr_gaming > 0;

	/*
	 * Gaming/interactive tasks: prefer V-Cache CCD with SMT-idle cores
	 */
	if (tctx->wants_vcache) {
		/* First try: SMT-idle core in V-Cache CCD */
		cpu = pick_idle_cpu_in_ccd(p, vcache_ccd, true);
		if (cpu >= 0) {
			if (prev_cctx->ccd != vcache_ccd)
				__sync_fetch_and_add(&nr_vcache_migrations, 1);
			goto dispatch;
		}
		/* Second try: any idle CPU in V-Cache CCD */
		cpu = pick_idle_cpu_in_ccd(p, vcache_ccd, false);
		if (cpu >= 0) {
			if (prev_cctx->ccd != vcache_ccd)
				__sync_fetch_and_add(&nr_vcache_migrations, 1);
			goto dispatch;
		}
	}

	/*
	 * Core compaction: when gaming tasks are on V-Cache CCD,
	 * steer batch tasks to other CCDs to avoid contention.
	 */
	if (!tctx->wants_vcache && vcache_has_gaming && prev_cctx->ccd == vcache_ccd) {
		/* Try non-V-Cache CCDs first */
		for (u32 i = 0; i < nr_ccds && i < MAX_CCDS; i++) {
			if (i == vcache_ccd)
				continue;
			cpu = pick_idle_cpu_in_ccd(p, i, false);
			if (cpu >= 0) {
				__sync_fetch_and_add(&nr_compaction_overflows, 1);
				goto dispatch;
			}
		}
	}

	/* Try to stay on current CCD */
	target_ccd = prev_cctx->ccd;
	cpu = pick_idle_cpu_in_ccd(p, target_ccd, tctx->wants_vcache);
	if (cpu >= 0) {
		__sync_fetch_and_add(&nr_ccd_local, 1);
		goto dispatch;
	}

	/* Try other CCDs */
	for (u32 i = 0; i < nr_ccds && i < MAX_CCDS; i++) {
		if (i == target_ccd)
			continue;
		cpu = pick_idle_cpu_in_ccd(p, i, false);
		if (cpu >= 0) {
			__sync_fetch_and_add(&nr_ccd_cross, 1);
			goto dispatch;
		}
	}

	/* Fallback to default selection */
	cpu = scx_bpf_select_cpu_dfl(p, prev_cpu, wake_flags, &is_idle);
	if (is_idle)
		goto dispatch;

	return cpu;

dispatch:
	/* Direct dispatch to the selected idle CPU */
	scx_bpf_dsq_insert(p, SCX_DSQ_LOCAL, slice_ns, 0);
	__sync_fetch_and_add(&nr_direct_dispatched, 1);
	return cpu;
}

/*
 * ops.enqueue - Enqueue task to CCD-specific dispatch queue
 *
 * For gaming tasks that couldn't find an idle CPU in select_cpu,
 * try to kick a lower-priority task from the V-Cache CCD.
 */
void BPF_STRUCT_OPS(ghostbrew_enqueue, struct task_struct *p, u64 enq_flags)
{
	struct task_ctx *tctx;
	struct cpu_ctx *cctx;
	u64 vtime = 0;
	u64 dsq_id = FALLBACK_DSQ;
	s32 cpu, kick_cpu;

	__sync_fetch_and_add(&nr_enqueued, 1);

	tctx = get_task_ctx(p);

	/* Determine target CCD DSQ */
	cpu = scx_bpf_task_cpu(p);
	cctx = get_cpu_ctx(cpu);
	if (cctx) {
		/* Use CCD-specific DSQ */
		if (tctx && tctx->wants_vcache) {
			dsq_id = ccd_to_dsq(vcache_ccd);
		} else {
			dsq_id = ccd_to_dsq(cctx->ccd);
		}

		/* Track which CCD the task is on */
		if (tctx)
			tctx->last_ccd = cctx->ccd;
	}

	if (tctx) {
		/* BORE-style priority: lower vtime = higher priority */
		if (tctx->is_gaming) {
			vtime = 0;  /* Highest priority for gaming */
			__sync_fetch_and_add(&nr_gaming_tasks, 1);

			/*
			 * Kick preemption: if gaming task needs V-Cache CCD,
			 * find a lower-priority task to preempt.
			 */
			if (tctx->wants_vcache) {
				kick_cpu = find_kick_victim_in_ccd(vcache_ccd, PRIO_GAMING);
				if (kick_cpu >= 0) {
					scx_bpf_kick_cpu(kick_cpu, SCX_KICK_PREEMPT);
					__sync_fetch_and_add(&nr_preempt_kicks, 1);
				}
			}
		} else if (tctx->is_interactive) {
			vtime = tctx->burst_time / 1000;
			__sync_fetch_and_add(&nr_interactive_tasks, 1);
		} else {
			/* CPU hogs get penalized */
			vtime = tctx->burst_time / 100;
		}
	}

	scx_bpf_dsq_insert_vtime(p, dsq_id, slice_ns, vtime, enq_flags);
}

/*
 * ops.dispatch - Dispatch from CCD DSQ with locality preference
 */
void BPF_STRUCT_OPS(ghostbrew_dispatch, s32 cpu, struct task_struct *prev)
{
	struct cpu_ctx *cctx;
	u64 local_dsq;

	cctx = get_cpu_ctx(cpu);
	if (!cctx) {
		/* Fallback if no CPU context */
		scx_bpf_dsq_move_to_local(FALLBACK_DSQ);
		__sync_fetch_and_add(&nr_dispatched, 1);
		return;
	}

	/* First try local CCD's DSQ */
	local_dsq = ccd_to_dsq(cctx->ccd);
	if (scx_bpf_dsq_move_to_local(local_dsq)) {
		__sync_fetch_and_add(&nr_dispatched, 1);
		return;
	}

	/* For V-Cache CPUs, also check V-Cache DSQ specifically */
	if (cctx->is_vcache) {
		u64 vcache_dsq = ccd_to_dsq(vcache_ccd);
		if (vcache_dsq != local_dsq && scx_bpf_dsq_move_to_local(vcache_dsq)) {
			__sync_fetch_and_add(&nr_dispatched, 1);
			return;
		}
	}

	/* Try other CCD DSQs */
	for (u32 i = 0; i < nr_ccds && i < MAX_CCDS; i++) {
		u64 dsq_id = ccd_to_dsq(i);
		if (dsq_id == local_dsq)
			continue;
		if (scx_bpf_dsq_move_to_local(dsq_id)) {
			__sync_fetch_and_add(&nr_dispatched, 1);
			return;
		}
	}

	/* Finally try fallback DSQ */
	if (scx_bpf_dsq_move_to_local(FALLBACK_DSQ)) {
		__sync_fetch_and_add(&nr_dispatched, 1);
	}
}

/*
 * ops.running - Task started running
 *
 * Updates per-CCD load counters and per-CPU running state.
 */
void BPF_STRUCT_OPS(ghostbrew_running, struct task_struct *p)
{
	struct task_ctx *tctx;
	struct cpu_ctx *cctx;
	struct ccd_load *load;
	struct cpu_run_state *state;
	s32 cpu;
	u32 key;
	u64 now = bpf_ktime_get_ns();

	tctx = get_task_ctx(p);
	if (tctx)
		tctx->last_run_at = now;

	cpu = scx_bpf_task_cpu(p);
	cctx = get_cpu_ctx(cpu);
	if (!cctx)
		return;

	/* Update per-CCD load */
	load = get_ccd_load(cctx->ccd);
	if (load) {
		__sync_fetch_and_add(&load->nr_tasks, 1);
		if (tctx && tctx->is_gaming)
			__sync_fetch_and_add(&load->nr_gaming, 1);
	}

	/* Update per-CPU run state for preemption decisions */
	key = cpu;
	state = bpf_map_lookup_elem(&cpu_run_states, &key);
	if (state) {
		state->started_at = now;
		state->pid = p->pid;

		if (tctx && tctx->is_gaming)
			state->priority_class = PRIO_GAMING;
		else if (tctx && tctx->is_interactive)
			state->priority_class = PRIO_INTERACTIVE;
		else
			state->priority_class = PRIO_BATCH;
	}
}

/*
 * ops.stopping - Task stopped running
 *
 * Updates burst tracking and decrements per-CCD load counters.
 */
void BPF_STRUCT_OPS(ghostbrew_stopping, struct task_struct *p, bool runnable)
{
	struct task_ctx *tctx;
	struct cpu_ctx *cctx;
	struct ccd_load *load;
	struct cpu_run_state *state;
	u64 now, delta;
	s32 cpu;
	u32 key;

	tctx = get_task_ctx(p);
	now = bpf_ktime_get_ns();

	/* Update burst tracking */
	if (tctx && tctx->last_run_at > 0) {
		delta = now - tctx->last_run_at;

		if (runnable) {
			/* Still runnable - accumulate burst time */
			tctx->burst_time += delta;
		} else {
			/* Sleeping - reset burst time */
			tctx->burst_time = 0;
		}
	}

	/* Update per-CCD load */
	cpu = scx_bpf_task_cpu(p);
	cctx = get_cpu_ctx(cpu);
	if (cctx) {
		load = get_ccd_load(cctx->ccd);
		if (load) {
			if (load->nr_tasks > 0)
				__sync_fetch_and_sub(&load->nr_tasks, 1);
			if (tctx && tctx->is_gaming && load->nr_gaming > 0)
				__sync_fetch_and_sub(&load->nr_gaming, 1);
		}
	}

	/* Clear per-CPU run state */
	key = cpu;
	state = bpf_map_lookup_elem(&cpu_run_states, &key);
	if (state && state->pid == p->pid) {
		state->priority_class = PRIO_BATCH;
		state->pid = 0;
	}
}

/*
 * ops.init - Initialize scheduler and per-CCD DSQs
 */
s32 BPF_STRUCT_OPS_SLEEPABLE(ghostbrew_init)
{
	s32 ret;

	/* Create fallback DSQ */
	ret = scx_bpf_create_dsq(FALLBACK_DSQ, -1);
	if (ret)
		return ret;

	/* Create per-CCD DSQs */
	for (u32 i = 0; i < nr_ccds && i < MAX_CCDS; i++) {
		ret = scx_bpf_create_dsq(ccd_to_dsq(i), -1);
		if (ret)
			return ret;
	}

	return 0;
}

/*
 * ops.exit - Cleanup scheduler
 */
void BPF_STRUCT_OPS(ghostbrew_exit, struct scx_exit_info *ei)
{
	UEI_RECORD(uei, ei);
}

/*
 * Scheduler operations
 */
SCX_OPS_DEFINE(ghostbrew_ops,
	       .select_cpu	= (void *)ghostbrew_select_cpu,
	       .enqueue		= (void *)ghostbrew_enqueue,
	       .dispatch	= (void *)ghostbrew_dispatch,
	       .running		= (void *)ghostbrew_running,
	       .stopping	= (void *)ghostbrew_stopping,
	       .init		= (void *)ghostbrew_init,
	       .exit		= (void *)ghostbrew_exit,
	       .timeout_ms	= 5000,
	       .name		= "ghostbrew");
