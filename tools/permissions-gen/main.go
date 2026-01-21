package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"slices"
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
	Bash opencodeBashConfig `yaml:"bash"`
}

type opencodeBashConfig struct {
	Default string   `yaml:"default"`
	Allow   []string `yaml:"allow"`
	Ask     []string `yaml:"ask"`
	Deny    []string `yaml:"deny"`
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

	dataPath, err = resolveOrDefault(dataPath, root, defaultDataPath)
	if err != nil {
		return err
	}

	claudePath, err = resolveOrDefault(claudePath, root, defaultClaudePath)
	if err != nil {
		return err
	}

	codexPath, err = resolveOrDefault(codexPath, root, defaultCodexPath)
	if err != nil {
		return err
	}

	opencodePath, err = resolveOrDefault(opencodePath, root, defaultOpencodePath)
	if err != nil {
		return err
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
	if !fileExists(path) {
		logSkip("skipping claude: %s not found", path)
		return nil
	}

	contents, err := os.ReadFile(path)
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

	if err := os.WriteFile(path, []byte(updated), 0o644); err != nil {
		return fmt.Errorf("write target: %w", err)
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
		if trimmed, ok := strings.CutPrefix(line, "  "); ok {
			inner[i] = trimmed
			continue
		}
		inner[i] = line
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

func buildCodexDecisionRules(decision string, commands []string) []codexRule {
	var order []string
	type group struct {
		prefix []string
		alts   []string
		seen   map[string]struct{}
	}
	groups := make(map[string]*group)
	singles := make(map[string][]string)

	for _, cmd := range commands {
		tokens := strings.Fields(cmd)
		if len(tokens) == 0 {
			continue
		}
		if len(tokens) == 1 {
			key := "single|" + tokens[0]
			if _, ok := singles[key]; !ok {
				singles[key] = tokens
				order = append(order, key)
			}
			continue
		}

		prefix := strings.Join(tokens[:len(tokens)-1], "\x1f")
		key := fmt.Sprintf("group|%d|%s", len(tokens), prefix)
		if _, ok := groups[key]; !ok {
			groups[key] = &group{
				prefix: tokens[:len(tokens)-1],
				alts:   []string{},
				seen:   make(map[string]struct{}),
			}
			order = append(order, key)
		}

		last := tokens[len(tokens)-1]
		if _, ok := groups[key].seen[last]; ok {
			continue
		}
		groups[key].seen[last] = struct{}{}
		groups[key].alts = append(groups[key].alts, last)
	}

	var rules []codexRule
	for _, key := range order {
		if tokens, ok := singles[key]; ok {
			rules = append(rules, codexRule{
				PatternPrefix: tokens,
				Decision:      decision,
				Match:         strings.Join(tokens, " "),
			})
			continue
		}
		group := groups[key]
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
	if !fileExists(path) {
		logSkip("skipping opencode: %s not found", path)
		return nil
	}

	contents, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("read opencode config: %w", err)
	}

	rules := buildOpencodeRules(cfg)
	bashJSON := renderOpencodeBashJSON(rules)

	updated, err := replaceOpencodeBash(string(contents), bashJSON)
	if err != nil {
		return err
	}

	if updated == string(contents) {
		return nil
	}

	if err := os.WriteFile(path, []byte(updated), 0o644); err != nil {
		return fmt.Errorf("write opencode config: %w", err)
	}

	return nil
}

func buildOpencodeRules(cfg config) []opencodeRule {
	defaultDecision := strings.TrimSpace(cfg.Opencode.Bash.Default)
	if defaultDecision == "" {
		defaultDecision = "allow"
	}

	rules := []opencodeRule{{Pattern: "*", Decision: defaultDecision}}
	rules = append(rules, buildOpencodeDecisionRules("allow", cfg.Bash.Allow, cfg.Opencode.Bash.Allow)...)
	rules = append(rules, buildOpencodeDecisionRules("ask", cfg.Bash.Ask, cfg.Opencode.Bash.Ask)...)
	rules = append(rules, buildOpencodeDecisionRules("deny", cfg.Bash.Deny, cfg.Opencode.Bash.Deny)...)
	return rules
}

func buildOpencodeDecisionRules(decision string, common, specific []string) []opencodeRule {
	patterns := expandOpencodePatterns(append(common, specific...))
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
		if containsWildcard(trimmed) {
			out, seen = appendUnique(out, seen, trimmed)
			continue
		}
		out, seen = appendUnique(out, seen, trimmed)
		out, seen = appendUnique(out, seen, trimmed+" *")
	}
	return out
}

func containsWildcard(value string) bool {
	return strings.ContainsAny(value, "*?")
}

func renderOpencodeBashJSON(rules []opencodeRule) string {
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

func replaceOpencodeBash(contents, bashJSON string) (string, error) {
	_, permStart, permEnd, err := findObjectForKey(contents, "permission")
	if err != nil {
		return "", err
	}

	bashKeyPos, bashStart, bashEnd, err := findKeyValueInObject(contents, permStart, permEnd, "bash")
	if err != nil {
		return "", err
	}

	indent := lineIndentForPos(contents, bashKeyPos)
	replacement := indentMultilineValue(bashJSON, indent)

	return contents[:bashStart] + replacement + contents[bashEnd+1:], nil
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

func findObjectForKey(contents, key string) (int, int, int, error) {
	depth := 0
	for i := 0; i < len(contents); i++ {
		switch contents[i] {
		case '"':
			token, end, err := scanString(contents, i)
			if err != nil {
				return 0, 0, 0, err
			}
			if depth == 1 && token == key {
				keyPos := i
				j := skipSpaces(contents, end+1)
				if j >= len(contents) || contents[j] != ':' {
					return 0, 0, 0, fmt.Errorf("%s key missing colon", key)
				}
				j = skipSpaces(contents, j+1)
				if j >= len(contents) || contents[j] != '{' {
					return 0, 0, 0, fmt.Errorf("%s value must be object", key)
				}
				objEnd, err := findMatchingBrace(contents, j)
				if err != nil {
					return 0, 0, 0, err
				}
				return keyPos, j, objEnd, nil
			}
			i = end
		case '{':
			depth++
		case '}':
			depth--
		}
	}
	return 0, 0, 0, fmt.Errorf("%s object not found", key)
}

func findKeyValueInObject(contents string, objStart, objEnd int, key string) (int, int, int, error) {
	depth := 0
	for i := objStart; i <= objEnd; i++ {
		switch contents[i] {
		case '"':
			token, end, err := scanString(contents, i)
			if err != nil {
				return 0, 0, 0, err
			}
			if depth == 1 && token == key {
				keyPos := i
				j := skipSpaces(contents, end+1)
				if j >= len(contents) || contents[j] != ':' {
					return 0, 0, 0, fmt.Errorf("%s key missing colon", key)
				}
				j = skipSpaces(contents, j+1)
				if j >= len(contents) {
					return 0, 0, 0, fmt.Errorf("%s missing value", key)
				}
				valueEnd, err := findValueEnd(contents, j)
				if err != nil {
					return 0, 0, 0, err
				}
				return keyPos, j, valueEnd, nil
			}
			i = end
		case '{':
			depth++
		case '}':
			depth--
		}
	}
	return 0, 0, 0, fmt.Errorf("%s key not found", key)
}

func findValueEnd(contents string, start int) (int, error) {
	switch contents[start] {
	case '{':
		return findMatchingBrace(contents, start)
	case '[':
		return findMatchingBracket(contents, start)
	case '"':
		_, end, err := scanString(contents, start)
		return end, err
	default:
		for i := start; i < len(contents); i++ {
			switch contents[i] {
			case ',', '\n', '\r', '\t', ' ':
				return i - 1, nil
			case '}':
				return i - 1, nil
			}
		}
		return len(contents) - 1, nil
	}
}

func findMatchingBrace(contents string, start int) (int, error) {
	if contents[start] != '{' {
		return 0, fmt.Errorf("expected object start at %d", start)
	}
	depth := 0
	for i := start; i < len(contents); i++ {
		switch contents[i] {
		case '"':
			_, end, err := scanString(contents, i)
			if err != nil {
				return 0, err
			}
			i = end
		case '{':
			depth++
		case '}':
			depth--
			if depth == 0 {
				return i, nil
			}
		}
	}
	return 0, fmt.Errorf("unterminated object starting at %d", start)
}

func findMatchingBracket(contents string, start int) (int, error) {
	if contents[start] != '[' {
		return 0, fmt.Errorf("expected array start at %d", start)
	}
	depth := 0
	for i := start; i < len(contents); i++ {
		switch contents[i] {
		case '"':
			_, end, err := scanString(contents, i)
			if err != nil {
				return 0, err
			}
			i = end
		case '[':
			depth++
		case ']':
			depth--
			if depth == 0 {
				return i, nil
			}
		}
	}
	return 0, fmt.Errorf("unterminated array starting at %d", start)
}

func scanString(contents string, start int) (string, int, error) {
	if contents[start] != '"' {
		return "", 0, fmt.Errorf("expected string at %d", start)
	}
	escaped := false
	for i := start + 1; i < len(contents); i++ {
		if escaped {
			escaped = false
			continue
		}
		switch contents[i] {
		case '\\':
			escaped = true
		case '"':
			return contents[start+1 : i], i, nil
		}
	}
	return "", 0, fmt.Errorf("unterminated string at %d", start)
}

func skipSpaces(contents string, start int) int {
	for i := start; i < len(contents); i++ {
		switch contents[i] {
		case ' ', '\n', '\r', '\t':
			continue
		default:
			return i
		}
	}
	return len(contents)
}

func jsonString(value string) string {
	data, err := json.Marshal(value)
	if err != nil {
		return fmt.Sprintf("%q", value)
	}
	return string(data)
}
