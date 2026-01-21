package main

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"gopkg.in/yaml.v3"
)

func TestNormalizeList(t *testing.T) {
	got := normalizeList([]string{" foo ", "", "  ", "bar"}, false)
	want := []string{"foo", "bar"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("normalizeList() = %#v, want %#v", got, want)
	}
}

func TestToBashPatterns(t *testing.T) {
	got := toBashPatterns([]string{" foo ", "", "bar"})
	want := []string{"Bash(foo:*)", "Bash(bar:*)"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("toBashPatterns() = %#v, want %#v", got, want)
	}
}

func TestExpandWithBash_NoBashValues(t *testing.T) {
	got := expandWithBash([]string{" a "}, nil)
	want := []string{"a"}
	if got == nil {
		t.Fatal("expandWithBash() returned nil slice")
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("expandWithBash() = %#v, want %#v", got, want)
	}

	got = expandWithBash(nil, nil)
	if got == nil {
		t.Fatal("expandWithBash(nil, nil) returned nil slice")
	}
	if len(got) != 0 {
		t.Fatalf("expandWithBash(nil, nil) = %#v, want empty slice", got)
	}
}

func TestExpandWithBash_NoSentinel(t *testing.T) {
	got := expandWithBash([]string{"git status", "ls"}, []string{"git"})
	want := []string{"git status", "ls", "Bash(git:*)"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("expandWithBash() = %#v, want %#v", got, want)
	}
}

func TestExpandWithBash_WithSentinel(t *testing.T) {
	got := expandWithBash([]string{"alpha", bashSentinel, "beta", bashSentinel, "alpha"}, []string{"git", "ls"})
	want := []string{"alpha", "Bash(git:*)", "Bash(ls:*)", "beta"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("expandWithBash() = %#v, want %#v", got, want)
	}
}

func TestBuildClaudePermissions(t *testing.T) {
	cfg := config{
		Bash: bashConfig{
			Allow: []string{"git"},
			Ask:   []string{"cp"},
			Deny:  []string{"rm"},
		},
		Claude: claudeConfig{
			Allow:                 []string{"foo", bashSentinel},
			Ask:                   nil,
			Deny:                  []string{"   "},
			AdditionalDirectories: []string{" /tmp ", "", " /var "},
		},
	}

	got := buildClaudePermissions(cfg)
	want := claudePermissions{
		Allow:                 []string{"foo", "Bash(git:*)"},
		Ask:                   []string{"Bash(cp:*)"},
		Deny:                  []string{"Bash(rm:*)"},
		AdditionalDirectories: []string{"/tmp", "/var"},
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("buildClaudePermissions() = %#v, want %#v", got, want)
	}
}

func TestPermissionsLines(t *testing.T) {
	perm := claudePermissions{
		Allow:                 []string{"a"},
		Ask:                   []string{},
		Deny:                  []string{},
		AdditionalDirectories: []string{},
	}

	got, err := permissionsLines(perm)
	if err != nil {
		t.Fatalf("permissionsLines() error = %v", err)
	}
	want := []string{
		"\"allow\": [",
		"  \"a\"",
		"],",
		"\"ask\": [],",
		"\"deny\": [],",
		"\"additionalDirectories\": []",
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("permissionsLines() = %#v, want %#v", got, want)
	}
}

func TestReplacePermissionsBlock(t *testing.T) {
	input := strings.Join([]string{
		"before",
		"  " + startMarker,
		"  \"old\": true",
		"  " + endMarker,
		"after",
		"",
	}, "\n")
	perm := claudePermissions{
		Allow:                 []string{"a"},
		Ask:                   []string{},
		Deny:                  []string{},
		AdditionalDirectories: []string{},
	}

	got, err := replacePermissionsBlock(input, perm)
	if err != nil {
		t.Fatalf("replacePermissionsBlock() error = %v", err)
	}

	want := strings.Join([]string{
		"before",
		"  " + startMarker,
		"  \"allow\": [",
		"    \"a\"",
		"  ],",
		"  \"ask\": [],",
		"  \"deny\": [],",
		"  \"additionalDirectories\": []",
		"  " + endMarker,
		"after",
		"",
	}, "\n")

	if got != want {
		t.Fatalf("replacePermissionsBlock() output mismatch\n--- got ---\n%s\n--- want ---\n%s", got, want)
	}
}

func TestReplacePermissionsBlock_MissingMarkers(t *testing.T) {
	_, err := replacePermissionsBlock("no markers here", claudePermissions{})
	if err == nil {
		t.Fatal("replacePermissionsBlock() expected error for missing markers")
	}
}

func TestLineIndent_MarkerNotAlone(t *testing.T) {
	contents := "  prefix " + startMarker
	pos := strings.Index(contents, startMarker)
	if pos == -1 {
		t.Fatal("start marker not found in test contents")
	}
	_, err := lineIndent(contents, pos)
	if err == nil {
		t.Fatal("lineIndent() expected error for marker not on its own line")
	}
}

func TestBuildCodexDecisionRules_GroupingAndOrder(t *testing.T) {
	rules := buildCodexDecisionRules("allow", []string{"git status", "git log", "ls", "git status"})
	if len(rules) != 2 {
		t.Fatalf("buildCodexDecisionRules() len = %d, want 2", len(rules))
	}

	first := rules[0]
	if first.Decision != "allow" {
		t.Fatalf("first rule decision = %q, want %q", first.Decision, "allow")
	}
	if !reflect.DeepEqual(first.PatternPrefix, []string{"git"}) {
		t.Fatalf("first rule prefix = %#v, want %#v", first.PatternPrefix, []string{"git"})
	}
	if !reflect.DeepEqual(first.PatternAlts, []string{"status", "log"}) {
		t.Fatalf("first rule alts = %#v, want %#v", first.PatternAlts, []string{"status", "log"})
	}

	second := rules[1]
	if !reflect.DeepEqual(second.PatternPrefix, []string{"ls"}) || len(second.PatternAlts) != 0 {
		t.Fatalf("second rule = %#v, want prefix [\"ls\"] with no alts", second)
	}
}

func TestRenderCodexPattern_NoAlts(t *testing.T) {
	got := renderCodexPattern(codexRule{PatternPrefix: []string{"git", "status"}})
	want := "  pattern = [\"git\", \"status\"],\n"
	if got != want {
		t.Fatalf("renderCodexPattern() = %q, want %q", got, want)
	}
}

func TestRenderCodexPattern_WithAlts(t *testing.T) {
	got := renderCodexPattern(codexRule{PatternPrefix: []string{"git"}, PatternAlts: []string{"status", "log"}})
	want := "  pattern = [\"git\", [\n    \"status\",\n    \"log\",\n  ]],\n"
	if got != want {
		t.Fatalf("renderCodexPattern() = %q, want %q", got, want)
	}
}

func TestExpandOpencodePatterns(t *testing.T) {
	got := expandOpencodePatterns([]string{"git", "git", "rm *", "ls?", " "})
	want := []string{"git", "git *", "rm *", "ls?"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("expandOpencodePatterns() = %#v, want %#v", got, want)
	}
}

func TestBuildOpencodeDecisionRules_NoWildcard(t *testing.T) {
	got := buildOpencodeDecisionRules("allow", []string{"foo", "foo", "bar *"}, false)
	want := []opencodeRule{
		{Pattern: "foo", Decision: "allow"},
		{Pattern: "bar *", Decision: "allow"},
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("buildOpencodeDecisionRules() = %#v, want %#v", got, want)
	}
}

func TestReplaceOpencodePermissions_WithMarkers(t *testing.T) {
	input := strings.Join([]string{
		"{",
		"  \"permission\": {",
		"    " + startMarker,
		"    \"old\": \"value\"",
		"    " + endMarker,
		"  }",
		"}",
		"",
	}, "\n")
	sections := []opencodeSection{
		{
			Name: "bash",
			Rules: []opencodeRule{
				{Pattern: "*", Decision: "ask"},
			},
		},
		{
			Name: "webfetch",
			Rules: []opencodeRule{
				{Pattern: "*", Decision: "allow"},
			},
		},
	}
	permissionsJSON := renderOpencodePermissionsJSON(sections)
	lines, err := opencodePermissionsLinesFromJSON(permissionsJSON)
	if err != nil {
		t.Fatalf("opencodePermissionsLinesFromJSON() error = %v", err)
	}

	got, err := replaceOpencodePermissions(input, permissionsJSON, lines)
	if err != nil {
		t.Fatalf("replaceOpencodePermissions() error = %v", err)
	}

	want := strings.Join([]string{
		"{",
		"  \"permission\": {",
		"    " + startMarker,
		"    \"bash\": {",
		"      \"*\": \"ask\"",
		"    },",
		"    \"webfetch\": {",
		"      \"*\": \"allow\"",
		"    }",
		"    " + endMarker,
		"  }",
		"}",
		"",
	}, "\n")
	if got != want {
		t.Fatalf("replaceOpencodePermissions() output mismatch\n--- got ---\n%s\n--- want ---\n%s", got, want)
	}
}

func TestReplaceOpencodePermissions_FallbackJSON(t *testing.T) {
	input := strings.Join([]string{
		"{",
		"  \"permission\": {",
		"    \"bash\": {",
		"      \"old\": \"value\"",
		"    },",
		"    \"other\": 1",
		"  }",
		"}",
		"",
	}, "\n")
	permissionsJSON := strings.Join([]string{
		"{",
		"  \"bash\": {",
		"    \"*\": \"ask\"",
		"  },",
		"  \"webfetch\": {",
		"    \"*\": \"allow\"",
		"  }",
		"}",
	}, "\n")
	lines, err := opencodePermissionsLinesFromJSON(permissionsJSON)
	if err != nil {
		t.Fatalf("opencodePermissionsLinesFromJSON() error = %v", err)
	}

	got, err := replaceOpencodePermissions(input, permissionsJSON, lines)
	if err != nil {
		t.Fatalf("replaceOpencodePermissions() error = %v", err)
	}

	want := strings.Join([]string{
		"{",
		"  \"permission\": {",
		"    \"bash\": {",
		"      \"*\": \"ask\"",
		"    },",
		"    \"webfetch\": {",
		"      \"*\": \"allow\"",
		"    }",
		"  }",
		"}",
		"",
	}, "\n")
	if got != want {
		t.Fatalf("replaceOpencodePermissions() output mismatch\n--- got ---\n%s\n--- want ---\n%s", got, want)
	}
}

func TestRenderOpencodePermissionsJSON_ScalarSection(t *testing.T) {
	sections := []opencodeSection{
		{
			Name: "bash",
			Rules: []opencodeRule{
				{Pattern: "*", Decision: "ask"},
			},
		},
		{
			Name:     "webfetch",
			Scalar:   "allow",
			IsScalar: true,
		},
	}

	got := renderOpencodePermissionsJSON(sections)
	want := strings.Join([]string{
		"{",
		"  \"bash\": {",
		"    \"*\": \"ask\"",
		"  },",
		"  \"webfetch\": \"allow\"",
		"}",
	}, "\n")
	if got != want {
		t.Fatalf("renderOpencodePermissionsJSON() = %q, want %q", got, want)
	}
}

func TestReplaceOpencodePermissions_MissingPermission(t *testing.T) {
	permissionsJSON := "{\n  \"bash\": {}\n}"
	lines, err := opencodePermissionsLinesFromJSON(permissionsJSON)
	if err != nil {
		t.Fatalf("opencodePermissionsLinesFromJSON() error = %v", err)
	}
	_, err = replaceOpencodePermissions("{}", permissionsJSON, lines)
	if err == nil {
		t.Fatal("replaceOpencodePermissions() expected error for missing permission object")
	}
}

func TestFindRepoRoot(t *testing.T) {
	root := t.TempDir()
	dataPath := filepath.Join(root, ".chezmoidata", "permissions.yaml")
	if err := os.MkdirAll(filepath.Dir(dataPath), 0o755); err != nil {
		t.Fatalf("MkdirAll() error = %v", err)
	}
	if err := os.WriteFile(dataPath, []byte(""), 0o644); err != nil {
		t.Fatalf("WriteFile() error = %v", err)
	}
	nested := filepath.Join(root, "a", "b")
	if err := os.MkdirAll(nested, 0o755); err != nil {
		t.Fatalf("MkdirAll() error = %v", err)
	}

	got, err := findRepoRoot(nested)
	if err != nil {
		t.Fatalf("findRepoRoot() error = %v", err)
	}
	if got != root {
		t.Fatalf("findRepoRoot() = %q, want %q", got, root)
	}

	_, err = findRepoRoot(t.TempDir())
	if err == nil {
		t.Fatal("findRepoRoot() expected error when repo root missing")
	}
}

func TestExpandHome(t *testing.T) {
	t.Setenv("HOME", "/tmp/testhome")

	got, err := expandHome("~")
	if err != nil {
		t.Fatalf("expandHome(~) error = %v", err)
	}
	want := "/tmp/testhome"
	if got != want {
		t.Fatalf("expandHome(~) = %q, want %q", got, want)
	}

	got, err = expandHome("~/dir")
	if err != nil {
		t.Fatalf("expandHome(~/dir) error = %v", err)
	}
	want = filepath.Join("/tmp/testhome", "dir")
	if got != want {
		t.Fatalf("expandHome(~/dir) = %q, want %q", got, want)
	}

	_, err = expandHome("~other")
	if err == nil {
		t.Fatal("expandHome(~other) expected error")
	}
}

func TestResolvePath(t *testing.T) {
	got, err := resolvePath("foo")
	if err != nil {
		t.Fatalf("resolvePath() error = %v", err)
	}
	want, err := filepath.Abs("foo")
	if err != nil {
		t.Fatalf("filepath.Abs() error = %v", err)
	}
	if got != want {
		t.Fatalf("resolvePath() = %q, want %q", got, want)
	}
}

func TestOpencodeSectionConfig_ScalarValue(t *testing.T) {
	input := []byte(strings.Join([]string{
		"opencode:",
		"  bash: ask",
		"  webfetch: allow",
	}, "\n"))

	var cfg config
	if err := yaml.Unmarshal(input, &cfg); err != nil {
		t.Fatalf("yaml.Unmarshal() error = %v", err)
	}

	if !cfg.Opencode.Bash.IsScalar || cfg.Opencode.Bash.Scalar != "ask" {
		t.Fatalf("opencode bash scalar = (%v, %q), want (%v, %q)", cfg.Opencode.Bash.IsScalar, cfg.Opencode.Bash.Scalar, true, "ask")
	}

	webfetch, ok := cfg.Opencode.Others["webfetch"]
	if !ok {
		t.Fatal("opencode webfetch section missing")
	}
	if !webfetch.IsScalar || webfetch.Scalar != "allow" {
		t.Fatalf("opencode webfetch scalar = (%v, %q), want (%v, %q)", webfetch.IsScalar, webfetch.Scalar, true, "allow")
	}
}
