package cmd

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"strings"

	"github.com/charmbracelet/bubbles/list"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

// Smart cache for AUR API and PKGBUILDs (session only)
var aurCache = make(map[string][]aurPkg)
var pkgbuildCache = make(map[string]string)

type aurPkg struct {
	Name, Version, Desc, Maintainer string
	Selected                        bool
	Deps                            []string
	Changelog                       string
	Comments                        []string
}

func (a aurPkg) Title() string       { return a.Name + " (" + a.Version + ")" }
func (a aurPkg) Description() string { return a.Desc }
func (a aurPkg) FilterValue() string { return a.Name }

type model struct {
	list     list.Model
	loading  bool
	error    string
	quitting bool
	showInfo bool
	showPKGB bool
	selected map[int]struct{}
	pkgb     string
	deps     []string
}

var docStyle = lipgloss.NewStyle().Margin(1, 2)

func initialModel() model {
	l := list.New([]list.Item{}, list.NewDefaultDelegate(), 0, 20)
	l.Title = "AUR Search"
	return model{list: l, selected: make(map[int]struct{})}
}

func (m model) Init() tea.Cmd {
	return nil
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "ctrl+c", "q":
			m.quitting = true
			return m, tea.Quit
		case "/":
			m.list.SetFilteringEnabled(true)
			return m, nil
		case "i":
			m.showInfo = !m.showInfo
			return m, nil
		case "p":
			m.showPKGB = !m.showPKGB
			if sel, ok := m.list.SelectedItem().(aurPkg); ok && m.showPKGB {
				pkgb, _ := getPKGBUILD(sel.Name)
				m.pkgb = pkgb
			}
			return m, nil
		case "d":
			if sel, ok := m.list.SelectedItem().(aurPkg); ok {
				m.deps = sel.Deps
			}
			return m, nil
		case " ":
			idx := m.list.Index()
			if _, ok := m.selected[idx]; ok {
				delete(m.selected, idx)
			} else {
				m.selected[idx] = struct{}{}
			}
			return m, nil
		case "enter":
			var pkgs []string
			for idx := range m.selected {
				if item, ok := m.list.Items()[idx].(aurPkg); ok {
					pkgs = append(pkgs, item.Name)
				}
			}
			if len(pkgs) == 0 {
				if sel, ok := m.list.SelectedItem().(aurPkg); ok {
					pkgs = append(pkgs, sel.Name)
				}
			}
			fmt.Printf("\n[ghostbrew] Installing: %v...\n", pkgs)
			InstallPackages(pkgs, InstallOptions{Parallel: 2})
			return m, tea.Quit
		}
	}
	l, cmd := m.list.Update(msg)
	m.list = l
	return m, cmd
}

func (m model) View() string {
	if m.quitting {
		return "Goodbye!"
	}
	if m.error != "" {
		return "Error: " + m.error
	}
	mainView := docStyle.Render(m.list.View())
	info := ""
	if m.showInfo {
		if sel, ok := m.list.SelectedItem().(aurPkg); ok {
			info = fmt.Sprintf("\n[Info]\nMaintainer: %s\nChangelog: %s\nComments: %v\n", sel.Maintainer, sel.Changelog, sel.Comments)
		}
	}
	pkgb := ""
	if m.showPKGB {
		pkgb = "\n[PKGBUILD Preview]\n" + m.pkgb
	}
	deps := ""
	if len(m.deps) > 0 {
		deps = "\n[Deps] " + strings.Join(m.deps, ", ")
		deps += "\n" + renderDepTree(m.deps, 0)
	}
	return mainView + info + pkgb + deps + "\n[j] down  [k] up  [/] search  [space] select  [enter] install  [i] info  [p] PKGBUILD  [d] deps  [q] quit"
}

// Dependency tree visualization (simple, recursive)
func renderDepTree(deps []string, level int) string {
	if len(deps) == 0 || level > 3 { // limit depth
		return ""
	}
	indent := strings.Repeat("  ", level)
	var out string
	for _, dep := range deps {
		out += fmt.Sprintf("%s- %s\n", indent, dep)
		// Optionally, fetch sub-deps here for a real tree (stubbed for now)
	}
	return out
}

func StartTUI() {
	fmt.Print("Search term: ")
	var term string
	fmt.Scanln(&term)
	pkgs, err := aurSearch(term)
	if err != nil {
		fmt.Println("AUR search failed:", err)
		return
	}
	items := make([]list.Item, len(pkgs))
	for i, p := range pkgs {
		items[i] = p
	}
	m := initialModel()
	m.list.SetItems(items)
	p := tea.NewProgram(m)
	if _, err := p.Run(); err != nil {
		fmt.Println("TUI error:", err)
	}
}

func aurSearch(term string) ([]aurPkg, error) {
	if pkgs, ok := aurCache[term]; ok {
		return pkgs, nil
	}
	resp, err := http.Get("https://aur.archlinux.org/rpc/?v=5&type=search&arg=" + term)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	var result struct {
		Results []struct {
			Name, Version, Description, Maintainer string
		}
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}
	pkgs := make([]aurPkg, len(result.Results))
	for i, r := range result.Results {
		pkgs[i] = aurPkg{r.Name, r.Version, r.Description, r.Maintainer, false, nil, "", nil}
	}
	// TODO: fetch deps, changelog, comments for each pkg
	aurCache[term] = pkgs
	return pkgs, nil
}

func getPKGBUILD(pkg string) (string, error) {
	if pkgb, ok := pkgbuildCache[pkg]; ok {
		return pkgb, nil
	}
	url := fmt.Sprintf("https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h=%s", pkg)
	resp, err := http.Get(url)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()
	data, err := os.ReadFile(resp.Request.URL.Path)
	if err != nil {
		return "", err
	}
	pkgb := string(data)
	pkgbuildCache[pkg] = pkgb
	return pkgb, nil
}
