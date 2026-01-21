package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"slices"
	"sort"
	"strings"

	"gopkg.in/yaml.v3"
)

const (
	startMarker = "{{/* PERMISSIONS:START */}}"
	endMarker   = "{{/* PERMISSIONS:END */}}"

	defaultDataPath     = ".chezmoidata/permissions.yaml"
	defaultClaudePath   = "dot_claude/settings.json.tmpl"
	defaultCodexPath    = "dot_codex/rules/default.rules"
	defaultOpencodePath = "dot_config/opencode/opencode.json"
)

type config struct {
	Bash     bashConfig     `yaml:"bash"`
	Claude   claudeConfig   `yaml:"claude"`
	Opencode opencodeConfig `yaml:"opencode"`
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

type opencodeConfig struct {
	Bash   opencodeSectionConfig            `yaml:"bash"`
	Others map[string]opencodeSectionConfig `yaml:",inline"`
}

type opencodeSectionConfig struct {
	Default  string   `yaml:"default"`
	Allow    []string `yaml:"allow"`
	Ask      []string `yaml:"ask"`
	Deny     []string `yaml:"deny"`
	Scalar   string   `yaml:"-"`
	IsScalar bool     `yaml:"-"`
}

func (c *opencodeSectionConfig) UnmarshalYAML(value *yaml.Node) error {
	if value.Kind == yaml.ScalarNode {
		var decision string
		if err := value.Decode(&decision); err != nil {
			return err
		}
		*c = opencodeSectionConfig{
			Scalar:   strings.TrimSpace(decision),
			IsScalar: true,
		}
		return nil
	}

	type raw opencodeSectionConfig
	var decoded raw
	if err := value.Decode(&decoded); err != nil {
		return err
	}
	*c = opencodeSectionConfig(decoded)
	return nil
}

const bashSentinel = "__BASH__"

var quiet bool

func main() {
	dataPath := flag.String("data", "", "path to permissions YAML")
	claudePath := flag.String("target", "", "path to settings.json.tmpl")
	codexPath := flag.String("codex", "", "path to default.rules")
	opencodePath := flag.String("opencode", "", "path to opencode.json")
	flag.BoolVar(&quiet, "quiet", false, "suppress skip messages")
	flag.BoolVar(&quiet, "q", false, "suppress skip messages (shorthand)")
	flag.Parse()

	if err := run(*dataPath, *claudePath, *codexPath, *opencodePath); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func logSkip(format string, args ...any) {
	if !quiet {
		fmt.Fprintf(os.Stderr, format+"\n", args...)
	}
}

func run(dataPath, claudePath, codexPath, opencodePath string) error {
	root, err := resolveRoot()
	if err != nil {
		return err
	}

	paths := []struct {
		value      *string
		defaultVal string
	}{
		{&dataPath, defaultDataPath},
		{&claudePath, defaultClaudePath},
		{&codexPath, defaultCodexPath},
		{&opencodePath, defaultOpencodePath},
	}
	for _, p := range paths {
		*p.value, err = resolveOrDefault(*p.value, root, p.defaultVal)
		if err != nil {
			return err
		}
	}

	cfg, err := loadConfig(dataPath)
	if err != nil {
		return err
	}

	perm := buildClaudePermissions(cfg)

	if err := writeClaudePermissions(perm, claudePath); err != nil {
		return err
	}

	if err := writeCodexRules(cfg, codexPath); err != nil {
		return err
	}

	if err := writeOpencodePermissions(cfg, opencodePath); err != nil {
		return err
	}

	return nil
}

func writeClaudePermissions(perm claudePermissions, path string) error {
	return updateFileIfChanged(path, "skipping claude: %s not found", func(contents string) (string, error) {
		return replacePermissionsBlock(contents, perm)
	})
}

func updateFileIfChanged(path, skipMsg string, transform func(string) (string, error)) error {
	if !fileExists(path) {
		logSkip(skipMsg, path)
		return nil
	}

	contents, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("read file: %w", err)
	}

	updated, err := transform(string(contents))
	if err != nil {
		return err
	}

	if updated == string(contents) {
		return nil
	}

	if err := os.WriteFile(path, []byte(updated), 0o644); err != nil {
		return fmt.Errorf("write file: %w", err)
	}

	return nil
}

func resolveRoot() (string, error) {
	cwd, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("get working directory: %w", err)
	}
	return findRepoRoot(cwd)
}

func resolveOrDefault(path, root, defaultPath string) (string, error) {
	if path == "" {
		return filepath.Join(root, defaultPath), nil
	}
	return resolvePath(path)
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

func dirExists(path string) bool {
	info, err := os.Stat(path)
	return err == nil && info.IsDir()
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
		AdditionalDirectories: ensureSlice(normalizeList(cfg.Claude.AdditionalDirectories, false)),
	}
}

func replacePermissionsBlock(contents string, perm claudePermissions) (string, error) {
	start := strings.Index(contents, startMarker)
	end := strings.Index(contents, endMarker)

	if start != -1 && end != -1 && start < end {
		return replaceWithMarkers(contents, perm, start, end)
	}

	return replacePermissionsJSON(contents, perm)
}

func replaceWithMarkers(contents string, perm claudePermissions, start, end int) (string, error) {
	lines, err := permissionsLines(perm)
	if err != nil {
		return "", err
	}

	return replaceBlockWithLines(contents, start, end, lines)
}

func replaceBlockWithLines(contents string, start, end int, lines []string) (string, error) {
	indent, err := lineIndent(contents, start)
	if err != nil {
		return "", err
	}

	for i, line := range lines {
		lines[i] = indent + line
	}

	block := startMarker + "\n" + strings.Join(lines, "\n") + "\n" + indent + endMarker

	return contents[:start] + block + contents[end+len(endMarker):], nil
}

func replacePermissionsJSON(contents string, perm claudePermissions) (string, error) {
	keyPos, objStart, objEnd, err := findObjectForKey(contents, "permissions")
	if err != nil {
		return "", fmt.Errorf("permissions object not found: %w", err)
	}

	data, err := json.MarshalIndent(perm, "", "  ")
	if err != nil {
		return "", fmt.Errorf("marshal permissions: %w", err)
	}

	indent := lineIndentForPos(contents, keyPos)
	replacement := indentMultilineValue(string(data), indent)

	return contents[:objStart] + replacement + contents[objEnd+1:], nil
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
	return innerJSONLines(string(data))
}

func innerJSONLines(data string) ([]string, error) {
	lines := strings.Split(data, "\n")
	if len(lines) < 2 {
		return nil, fmt.Errorf("unexpected json: too few lines")
	}

	inner := lines[1 : len(lines)-1]
	for i, line := range inner {
		if trimmed, ok := strings.CutPrefix(line, "  "); ok {
			inner[i] = trimmed
			continue
		}
		inner[i] = line
	}

	return inner, nil
}

func toBashPatterns(values []string) []string {
	normalized := normalizeList(values, false)
	out := make([]string, 0, len(normalized))
	for _, v := range normalized {
		out = append(out, fmt.Sprintf("Bash(%s:*)", v))
	}
	return out
}

func normalizeList(values []string, unique bool) []string {
	if unique {
		seen := make(map[string]struct{})
		var out []string
		for _, value := range values {
			trimmed := strings.TrimSpace(value)
			if trimmed == "" {
				continue
			}
			out, seen = appendUnique(out, seen, trimmed)
		}
		return out
	}
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
	normalized := normalizeList(values, false)
	bashPatterns := toBashPatterns(normalizeList(bashValues, false))

	if len(bashPatterns) == 0 {
		return ensureSlice(normalized)
	}

	if !slices.Contains(normalized, bashSentinel) {
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

type codexRule struct {
	PatternPrefix []string
	PatternAlts   []string
	Decision      string
	Match         string
}

func writeCodexRules(cfg config, path string) error {
	dir := filepath.Dir(path)
	if !dirExists(dir) {
		logSkip("skipping codex: %s not found", dir)
		return nil
	}

	rules := buildCodexRules(cfg)
	content := renderCodexRules(rules)
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		return fmt.Errorf("write codex rules: %w", err)
	}
	return nil
}

func buildCodexRules(cfg config) []codexRule {
	var rules []codexRule
	rules = append(rules, buildCodexDecisionRules("allow", cfg.Bash.Allow)...)
	rules = append(rules, buildCodexDecisionRules("prompt", cfg.Bash.Ask)...)
	rules = append(rules, buildCodexDecisionRules("forbidden", cfg.Bash.Deny)...)
	return rules
}

type commandGroup struct {
	prefix []string
	alts   []string
	seen   map[string]struct{}
}

type groupedCommands struct {
	order   []string
	groups  map[string]*commandGroup
	singles map[string][]string
}

func groupCommands(commands []string) groupedCommands {
	gc := groupedCommands{
		groups:  make(map[string]*commandGroup),
		singles: make(map[string][]string),
	}

	for _, cmd := range commands {
		tokens := strings.Fields(cmd)
		if len(tokens) == 0 {
			continue
		}
		if len(tokens) == 1 {
			key := "single|" + tokens[0]
			if _, ok := gc.singles[key]; !ok {
				gc.singles[key] = tokens
				gc.order = append(gc.order, key)
			}
			continue
		}

		prefix := strings.Join(tokens[:len(tokens)-1], "\x1f")
		key := fmt.Sprintf("group|%d|%s", len(tokens), prefix)
		if _, ok := gc.groups[key]; !ok {
			gc.groups[key] = &commandGroup{
				prefix: tokens[:len(tokens)-1],
				alts:   []string{},
				seen:   make(map[string]struct{}),
			}
			gc.order = append(gc.order, key)
		}

		last := tokens[len(tokens)-1]
		if _, ok := gc.groups[key].seen[last]; ok {
			continue
		}
		gc.groups[key].seen[last] = struct{}{}
		gc.groups[key].alts = append(gc.groups[key].alts, last)
	}

	return gc
}

func buildCodexDecisionRules(decision string, commands []string) []codexRule {
	gc := groupCommands(commands)

	var rules []codexRule
	for _, key := range gc.order {
		if tokens, ok := gc.singles[key]; ok {
			rules = append(rules, codexRule{
				PatternPrefix: tokens,
				Decision:      decision,
				Match:         strings.Join(tokens, " "),
			})
			continue
		}
		group := gc.groups[key]
		if group == nil {
			continue
		}
		if len(group.alts) == 1 {
			full := append([]string{}, group.prefix...)
			full = append(full, group.alts[0])
			rules = append(rules, codexRule{
				PatternPrefix: full,
				Decision:      decision,
				Match:         strings.Join(full, " "),
			})
			continue
		}
		matchTokens := append([]string{}, group.prefix...)
		matchTokens = append(matchTokens, group.alts[0])
		rules = append(rules, codexRule{
			PatternPrefix: group.prefix,
			PatternAlts:   group.alts,
			Decision:      decision,
			Match:         strings.Join(matchTokens, " "),
		})
	}

	return rules
}

func renderCodexRules(rules []codexRule) string {
	var builder strings.Builder
	builder.WriteString("# ~/.codex/rules/default.rules\n")
	builder.WriteString("# Generated by tools/permissions-gen. Do not edit by hand.\n\n")
	for i, rule := range rules {
		builder.WriteString("prefix_rule(\n")
		builder.WriteString(renderCodexPattern(rule))
		builder.WriteString(renderCodexDecision(rule.Decision))
		builder.WriteString(renderCodexMatch(rule.Match))
		builder.WriteString(")\n")
		if i < len(rules)-1 {
			builder.WriteString("\n")
		}
	}
	return builder.String()
}

func renderCodexPattern(rule codexRule) string {
	if len(rule.PatternAlts) == 0 {
		return fmt.Sprintf("  pattern = [%s],\n", joinQuoted(rule.PatternPrefix))
	}
	var builder strings.Builder
	builder.WriteString("  pattern = [")
	builder.WriteString(joinQuoted(rule.PatternPrefix))
	builder.WriteString(", [\n")
	for _, alt := range rule.PatternAlts {
		fmt.Fprintf(&builder, "    %q,\n", alt)
	}
	builder.WriteString("  ]],\n")
	return builder.String()
}

func renderCodexDecision(decision string) string {
	if decision == "" || decision == "allow" {
		return "  decision = \"allow\",\n"
	}
	return fmt.Sprintf("  decision = %q,\n", decision)
}

func renderCodexMatch(match string) string {
	if strings.TrimSpace(match) == "" {
		return ""
	}
	return fmt.Sprintf("  match = [%q],\n", match)
}

func joinQuoted(tokens []string) string {
	parts := make([]string, 0, len(tokens))
	for _, token := range tokens {
		parts = append(parts, fmt.Sprintf("%q", token))
	}
	return strings.Join(parts, ", ")
}

type opencodeRule struct {
	Pattern  string
	Decision string
}

func writeOpencodePermissions(cfg config, path string) error {
	sections := buildOpencodeSections(cfg)
	permissionsJSON := renderOpencodePermissionsJSON(sections)
	lines, err := opencodePermissionsLinesFromJSON(permissionsJSON)
	if err != nil {
		return err
	}

	return updateFileIfChanged(path, "skipping opencode: %s not found", func(contents string) (string, error) {
		return replaceOpencodePermissions(contents, permissionsJSON, lines)
	})
}

type opencodeSection struct {
	Name     string
	Rules    []opencodeRule
	Scalar   string
	IsScalar bool
}

func buildOpencodeSections(cfg config) []opencodeSection {
	var sections []opencodeSection
	if cfg.Opencode.Bash.IsScalar {
		sections = append(sections, opencodeSection{
			Name:     "bash",
			Scalar:   cfg.Opencode.Bash.Scalar,
			IsScalar: true,
		})
	} else {
		sections = append(sections, opencodeSection{
			Name:  "bash",
			Rules: buildOpencodeBashRules(cfg),
		})
	}

	if len(cfg.Opencode.Others) == 0 {
		return sections
	}

	var names []string
	for name := range cfg.Opencode.Others {
		if name == "bash" {
			continue
		}
		names = append(names, name)
	}
	sort.Strings(names)
	for _, name := range names {
		section := cfg.Opencode.Others[name]
		if section.IsScalar {
			sections = append(sections, opencodeSection{
				Name:     name,
				Scalar:   section.Scalar,
				IsScalar: true,
			})
			continue
		}
		rules := buildOpencodeSectionRules(section)
		sections = append(sections, opencodeSection{Name: name, Rules: rules})
	}
	return sections
}

func buildOpencodeBashRules(cfg config) []opencodeRule {
	return buildOpencodeRulesForSection(
		cfg.Opencode.Bash.Default,
		append(cfg.Bash.Allow, cfg.Opencode.Bash.Allow...),
		append(cfg.Bash.Ask, cfg.Opencode.Bash.Ask...),
		append(cfg.Bash.Deny, cfg.Opencode.Bash.Deny...),
		true,
	)
}

func buildOpencodeSectionRules(section opencodeSectionConfig) []opencodeRule {
	return buildOpencodeRulesForSection(
		section.Default,
		section.Allow,
		section.Ask,
		section.Deny,
		false,
	)
}

func buildOpencodeRulesForSection(defaultDecision string, allow, ask, deny []string, expand bool) []opencodeRule {
	decision := strings.TrimSpace(defaultDecision)
	if decision == "" {
		decision = "allow"
	}

	rules := []opencodeRule{{Pattern: "*", Decision: decision}}
	rules = append(rules, buildOpencodeDecisionRules("allow", allow, expand)...)
	rules = append(rules, buildOpencodeDecisionRules("ask", ask, expand)...)
	rules = append(rules, buildOpencodeDecisionRules("deny", deny, expand)...)
	return rules
}

func buildOpencodeDecisionRules(decision string, values []string, expand bool) []opencodeRule {
	var patterns []string
	if expand {
		patterns = expandOpencodePatterns(values)
	} else {
		patterns = normalizeList(values, true)
	}
	rules := make([]opencodeRule, 0, len(patterns))
	for _, pattern := range patterns {
		rules = append(rules, opencodeRule{
			Pattern:  pattern,
			Decision: decision,
		})
	}
	return rules
}

func expandOpencodePatterns(values []string) []string {
	seen := make(map[string]struct{})
	var out []string
	for _, value := range values {
		trimmed := strings.TrimSpace(value)
		if trimmed == "" {
			continue
		}
		out, seen = appendUnique(out, seen, trimmed)
		if !containsWildcard(trimmed) {
			out, seen = appendUnique(out, seen, trimmed+" *")
		}
	}
	return out
}

func containsWildcard(value string) bool {
	return strings.ContainsAny(value, "*?")
}

func renderOpencodeSectionJSON(rules []opencodeRule) string {
	var builder strings.Builder
	builder.WriteString("{\n")
	for i, rule := range rules {
		builder.WriteString("  ")
		builder.WriteString(jsonString(rule.Pattern))
		builder.WriteString(": ")
		builder.WriteString(jsonString(rule.Decision))
		if i < len(rules)-1 {
			builder.WriteString(",")
		}
		builder.WriteString("\n")
	}
	builder.WriteString("}")
	return builder.String()
}

func renderOpencodePermissionsJSON(sections []opencodeSection) string {
	var builder strings.Builder
	builder.WriteString("{\n")
	for i, section := range sections {
		builder.WriteString("  ")
		builder.WriteString(jsonString(section.Name))
		builder.WriteString(": ")
		if section.IsScalar {
			builder.WriteString(jsonString(section.Scalar))
		} else {
			builder.WriteString(indentMultilineValue(renderOpencodeSectionJSON(section.Rules), "  "))
		}
		if i < len(sections)-1 {
			builder.WriteString(",")
		}
		builder.WriteString("\n")
	}
	builder.WriteString("}")
	return builder.String()
}

func opencodePermissionsLinesFromJSON(permissionsJSON string) ([]string, error) {
	return innerJSONLines(permissionsJSON)
}

func replaceOpencodePermissions(contents, permissionsJSON string, lines []string) (string, error) {
	start := strings.Index(contents, startMarker)
	end := strings.Index(contents, endMarker)
	if start == -1 || end == -1 || start >= end {
		return replaceOpencodePermissionsJSON(contents, permissionsJSON)
	}
	return replaceBlockWithLines(contents, start, end, lines)
}

func replaceOpencodePermissionsJSON(contents, permissionsJSON string) (string, error) {
	permKeyPos, permStart, permEnd, err := findObjectForKey(contents, "permission")
	if err != nil {
		return "", err
	}
	indent := lineIndentForPos(contents, permKeyPos)
	replacement := indentMultilineValue(permissionsJSON, indent)
	return contents[:permStart] + replacement + contents[permEnd+1:], nil
}

func indentMultilineValue(value, indent string) string {
	lines := strings.Split(value, "\n")
	for i := 1; i < len(lines); i++ {
		lines[i] = indent + lines[i]
	}
	return strings.Join(lines, "\n")
}

func lineIndentForPos(contents string, pos int) string {
	lineStart := strings.LastIndex(contents[:pos], "\n") + 1
	return contents[lineStart:pos]
}

func jsonString(value string) string {
	data, err := json.Marshal(value)
	if err != nil {
		return fmt.Sprintf("%q", value)
	}
	return string(data)
}
