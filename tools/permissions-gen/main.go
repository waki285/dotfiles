package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"gopkg.in/yaml.v3"
)

const (
	startMarker = "{{/* PERMISSIONS:START */}}"
	endMarker   = "{{/* PERMISSIONS:END */}}"

	defaultDataPath   = ".chezmoidata/permissions.yaml"
	defaultTargetPath = "dot_claude/settings.json.tmpl"
)

type config struct {
	Bash   bashConfig   `yaml:"bash"`
	Claude claudeConfig `yaml:"claude"`
}

type bashConfig struct {
	Allow []string `yaml:"allow"`
	Ask   []string `yaml:"ask"`
	Deny  []string `yaml:"deny"`
}

type claudeConfig struct {
	Allow                 []string `yaml:"allow"`
	Ask                   []string `yaml:"ask"`
	Deny                  []string `yaml:"deny"`
	AdditionalDirectories []string `yaml:"additionalDirectories"`
}

type claudePermissions struct {
	Allow                 []string `json:"allow"`
	Ask                   []string `json:"ask"`
	Deny                  []string `json:"deny"`
	AdditionalDirectories []string `json:"additionalDirectories"`
}

const bashSentinel = "__BASH__"

func main() {
	dataPath := flag.String("data", "", "path to permissions YAML")
	targetPath := flag.String("target", "", "path to settings.json.tmpl")
	flag.Parse()

	if err := run(*dataPath, *targetPath); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(dataPath, targetPath string) error {
	root, err := resolveRoot()
	if err != nil {
		return err
	}

	if dataPath == "" {
		dataPath = filepath.Join(root, defaultDataPath)
	} else {
		dataPath, err = resolvePath(dataPath)
		if err != nil {
			return err
		}
	}

	if targetPath == "" {
		targetPath = filepath.Join(root, defaultTargetPath)
	} else {
		targetPath, err = resolvePath(targetPath)
		if err != nil {
			return err
		}
	}

	cfg, err := loadConfig(dataPath)
	if err != nil {
		return err
	}

	perm := buildClaudePermissions(cfg)

	contents, err := os.ReadFile(targetPath)
	if err != nil {
		return fmt.Errorf("read target: %w", err)
	}

	updated, err := replacePermissionsBlock(string(contents), perm)
	if err != nil {
		return err
	}

	if updated == string(contents) {
		return nil
	}

	if err := os.WriteFile(targetPath, []byte(updated), 0o644); err != nil {
		return fmt.Errorf("write target: %w", err)
	}

	return nil
}

func resolveRoot() (string, error) {
	cwd, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("get working directory: %w", err)
	}

	root, err := findRepoRoot(cwd)
	if err != nil {
		return "", err
	}

	return root, nil
}

func resolvePath(path string) (string, error) {
	if strings.HasPrefix(path, "~") {
		expanded, err := expandHome(path)
		if err != nil {
			return "", err
		}
		path = expanded
	}
	if filepath.IsAbs(path) {
		return path, nil
	}
	abs, err := filepath.Abs(path)
	if err != nil {
		return "", fmt.Errorf("resolve path: %w", err)
	}
	return abs, nil
}

func findRepoRoot(start string) (string, error) {
	dir := start
	for {
		if fileExists(filepath.Join(dir, defaultDataPath)) {
			return dir, nil
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
	}
	return "", fmt.Errorf("could not locate repo root from %s", start)
}

func expandHome(path string) (string, error) {
	if !strings.HasPrefix(path, "~") {
		return path, nil
	}
	home, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("resolve home: %w", err)
	}
	if path == "~" {
		return home, nil
	}
	if strings.HasPrefix(path, "~/") {
		return filepath.Join(home, path[2:]), nil
	}
	return "", fmt.Errorf("unsupported home path: %s", path)
}

func fileExists(path string) bool {
	info, err := os.Stat(path)
	return err == nil && !info.IsDir()
}

func loadConfig(path string) (config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return config{}, fmt.Errorf("read data: %w", err)
	}

	var cfg config
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return config{}, fmt.Errorf("parse yaml: %w", err)
	}

	return cfg, nil
}

func buildClaudePermissions(cfg config) claudePermissions {
	allow := expandWithBash(cfg.Claude.Allow, cfg.Bash.Allow)
	ask := expandWithBash(cfg.Claude.Ask, cfg.Bash.Ask)
	deny := expandWithBash(cfg.Claude.Deny, cfg.Bash.Deny)

	return claudePermissions{
		Allow:                 allow,
		Ask:                   ensureSlice(ask),
		Deny:                  ensureSlice(deny),
		AdditionalDirectories: ensureSlice(normalizeList(cfg.Claude.AdditionalDirectories)),
	}
}

func replacePermissionsBlock(contents string, perm claudePermissions) (string, error) {
	start := strings.Index(contents, startMarker)
	end := strings.Index(contents, endMarker)
	if start == -1 || end == -1 || end < start {
		return "", fmt.Errorf("permission markers not found")
	}

	indent, err := lineIndent(contents, start)
	if err != nil {
		return "", err
	}

	lines, err := permissionsLines(perm)
	if err != nil {
		return "", err
	}

	for i, line := range lines {
		lines[i] = indent + line
	}

	block := startMarker + "\n" + strings.Join(lines, "\n") + "\n" + indent + endMarker

	return contents[:start] + block + contents[end+len(endMarker):], nil
}

func lineIndent(contents string, markerPos int) (string, error) {
	lineStart := strings.LastIndex(contents[:markerPos], "\n") + 1
	indent := contents[lineStart:markerPos]
	if strings.TrimSpace(indent) != "" {
		return "", fmt.Errorf("marker must be on its own line: %q", indent)
	}
	return indent, nil
}

func permissionsLines(perm claudePermissions) ([]string, error) {
	data, err := json.MarshalIndent(perm, "", "  ")
	if err != nil {
		return nil, fmt.Errorf("marshal permissions: %w", err)
	}

	lines := strings.Split(string(data), "\n")
	if len(lines) < 2 {
		return nil, fmt.Errorf("unexpected permissions json")
	}

	inner := lines[1 : len(lines)-1]
	for i, line := range inner {
		if strings.HasPrefix(line, "  ") {
			inner[i] = strings.TrimPrefix(line, "  ")
		} else {
			inner[i] = line
		}
	}

	return inner, nil
}

func toBashPatterns(values []string) []string {
	var out []string
	for _, value := range values {
		trimmed := strings.TrimSpace(value)
		if trimmed == "" {
			continue
		}
		out = append(out, fmt.Sprintf("Bash(%s:*)", trimmed))
	}
	return out
}

func normalizeList(values []string) []string {
	var out []string
	for _, value := range values {
		trimmed := strings.TrimSpace(value)
		if trimmed == "" {
			continue
		}
		out = append(out, trimmed)
	}
	return out
}

func expandWithBash(values []string, bashValues []string) []string {
	normalized := normalizeList(values)
	bashPatterns := toBashPatterns(normalizeList(bashValues))

	if len(bashPatterns) == 0 {
		return ensureSlice(normalized)
	}

	hasSentinel := false
	for _, item := range normalized {
		if item == bashSentinel {
			hasSentinel = true
			break
		}
	}

	if !hasSentinel {
		return mergeUnique(normalized, bashPatterns)
	}

	seen := make(map[string]struct{})
	var out []string
	for _, item := range normalized {
		if item == bashSentinel {
			for _, bashItem := range bashPatterns {
				out, seen = appendUnique(out, seen, bashItem)
			}
			continue
		}
		out, seen = appendUnique(out, seen, item)
	}
	return out
}

func mergeUnique(lists ...[]string) []string {
	seen := make(map[string]struct{})
	var out []string
	for _, list := range lists {
		for _, item := range list {
			out, seen = appendUnique(out, seen, item)
		}
	}
	return out
}

func appendUnique(list []string, seen map[string]struct{}, item string) ([]string, map[string]struct{}) {
	if item == "" {
		return list, seen
	}
	if _, ok := seen[item]; ok {
		return list, seen
	}
	seen[item] = struct{}{}
	return append(list, item), seen
}

func ensureSlice(values []string) []string {
	if values == nil {
		return []string{}
	}
	return values
}
