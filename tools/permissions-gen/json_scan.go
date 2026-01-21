package main

import "fmt"

func findObjectForKey(contents, key string) (int, int, int, error) {
	keyPos, valueStart, err := findKeyInRange(contents, 0, len(contents)-1, key)
	if err != nil {
		return 0, 0, 0, fmt.Errorf("%s object not found: %w", key, err)
	}
	if contents[valueStart] != '{' {
		return 0, 0, 0, fmt.Errorf("%s value must be object", key)
	}
	valueEnd, err := findMatchingBrace(contents, valueStart)
	if err != nil {
		return 0, 0, 0, err
	}
	return keyPos, valueStart, valueEnd, nil
}

func findKeyInRange(contents string, start, end int, key string) (int, int, error) {
	depth := 0
	for i := start; i <= end; i++ {
		switch contents[i] {
		case '"':
			token, strEnd, err := scanString(contents, i)
			if err != nil {
				return 0, 0, err
			}
			if depth == 1 && token == key {
				keyPos := i
				j := skipSpaces(contents, strEnd+1)
				if j >= len(contents) || contents[j] != ':' {
					return 0, 0, fmt.Errorf("%s key missing colon", key)
				}
				j = skipSpaces(contents, j+1)
				if j >= len(contents) {
					return 0, 0, fmt.Errorf("%s missing value", key)
				}
				return keyPos, j, nil
			}
			i = strEnd
		case '{':
			depth++
		case '}':
			depth--
		}
	}
	return 0, 0, fmt.Errorf("key %q not found", key)
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
	return findMatchingDelimiter(contents, start, '{', '}', "object")
}

func findMatchingBracket(contents string, start int) (int, error) {
	return findMatchingDelimiter(contents, start, '[', ']', "array")
}

func findMatchingDelimiter(contents string, start int, open, close byte, name string) (int, error) {
	if contents[start] != open {
		return 0, fmt.Errorf("expected %s start at %d", name, start)
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
		case open:
			depth++
		case close:
			depth--
			if depth == 0 {
				return i, nil
			}
		}
	}
	return 0, fmt.Errorf("unterminated %s starting at %d", name, start)
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
