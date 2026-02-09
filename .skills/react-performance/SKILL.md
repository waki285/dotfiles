---
name: react-performance
description: React/Next.js performance best practices and anti-pattern corrections. Use when writing or reviewing React components, Next.js pages, Server Components, or client-side data fetching code. Covers async waterfall elimination, RSC serialization pitfalls, re-render optimization, hydration issues, and bundle size reduction. Triggers on React, Next.js, RSC, Server Components, Suspense, useMemo, useEffect, or performance-related frontend work.
---

Curated React/Next.js performance rules. Only non-obvious patterns that AI tends to get wrong are included.

## 1. Async & Data Fetching

### Suspense boundaries: don't block the whole page

Move data fetching into child async components wrapped in `<Suspense>`. Share a single Promise across siblings via `use()`.

```tsx
// BAD: entire page blocked
async function Page() {
  const data = await fetchData()
  return <div><Header /><DataView data={data} /><Footer /></div>
}

// GOOD: layout renders immediately, data streams in
function Page() {
  return (
    <div>
      <Header />
      <Suspense fallback={<Skeleton />}>
        <DataView />
      </Suspense>
      <Footer />
    </div>
  )
}
async function DataView() {
  const data = await fetchData()
  return <div>{data.content}</div>
}
```

Skip Suspense when: data affects layout positioning, content is SEO-critical above the fold, or the query is trivially fast.

### Defer await to the branch that needs it

```tsx
// BAD
async function handle(id: string, skip: boolean) {
  const data = await fetch(id)  // blocks even when skipping
  if (skip) return { skipped: true }
  return process(data)
}

// GOOD
async function handle(id: string, skip: boolean) {
  if (skip) return { skipped: true }
  const data = await fetch(id)
  return process(data)
}
```

## 2. Server Components & RSC

### React.cache(): use primitives, not objects

`React.cache()` compares arguments with `Object.is`. Inline objects always miss.

```tsx
// BAD: always cache miss
const getUser = cache(async (params: { uid: number }) => { ... })
getUser({ uid: 1 })
getUser({ uid: 1 })  // miss - new object reference

// GOOD: cache hit
const getUser = cache(async (uid: number) => { ... })
getUser(1)
getUser(1)  // hit
```

Next.js `fetch` has built-in deduplication. Use `React.cache()` for DB queries, auth checks, computations.

### Avoid duplicate RSC serialization

RSC deduplicates by object reference. Array transforms (`.toSorted()`, `.filter()`, `.map()`, `[...arr]`) create new references, duplicating primitive arrays in the payload.

```tsx
// BAD: serializes usernames twice (original + sorted copy)
<ClientList usernames={usernames} sorted={usernames.toSorted()} />

// GOOD: transform on client
<ClientList usernames={usernames} />
// client: const sorted = useMemo(() => [...usernames].sort(), [usernames])
```

High impact for `string[]`/`number[]`. Low impact for `object[]` (nested objects dedup by reference).

### Server Actions: always authenticate inside

Server Actions can be invoked directly, bypassing middleware/layout guards. Always verify auth inside each action.

### Next.js after() for non-blocking side effects

Use `after()` for logging, analytics, cache warming. Runs even if response fails or redirects.

```tsx
import { after } from 'next/server'

export async function POST(req: Request) {
  const result = await processRequest(req)
  after(async () => {
    await logAnalytics(result)
  })
  return Response.json(result)
}
```

## 3. Re-render Optimization

### Derive state during render, not in useEffect

Never store computed values in state and sync with useEffect. Derive inline.

```tsx
// BAD: extra render + state drift
const [fullName, setFullName] = useState('')
useEffect(() => { setFullName(first + ' ' + last) }, [first, last])

// GOOD
const fullName = first + ' ' + last
```

Reference: [You Might Not Need an Effect](https://react.dev/learn/you-might-not-need-an-effect)

### Put interaction logic in event handlers, not effects

Don't model user actions as state + effect. Run side effects directly in handlers.

```tsx
// BAD
const [submitted, setSubmitted] = useState(false)
useEffect(() => { if (submitted) post('/api/register') }, [submitted, theme])

// GOOD
function handleSubmit() { post('/api/register') }
```

### Don't useMemo trivial expressions

`useMemo(() => a + b, [a, b])` costs more than `a + b`. Only memo expensive computations.

### Default prop values break memo()

Inline defaults for non-primitive optional props create new references each render.

```tsx
// BAD: memo broken - new function on every render
const Button = memo(({ onClick = () => {} }) => <button onClick={onClick} />)

// GOOD: stable default
const NOOP = () => {}
const Button = memo(({ onClick = NOOP }) => <button onClick={onClick} />)
```

### Subscribe to derived booleans, not continuous values

```tsx
// BAD: re-renders on every pixel of resize
const [width, setWidth] = useState(window.innerWidth)

// GOOD: re-renders only on breakpoint crossing
const isMobile = useMediaQuery('(max-width: 767px)')
```

### Defer reads: don't subscribe to state only used in callbacks

```tsx
// BAD: re-renders on every searchParams change
const searchParams = useSearchParams()
const handleClick = () => { track(searchParams.get('ref')) }

// GOOD: read on demand
const handleClick = () => {
  const ref = new URLSearchParams(window.location.search).get('ref')
  track(ref)
}
```

## 4. Rendering

### Conditional rendering: ternary over &&

`{count && <List />}` renders `0` when count is 0. Use ternary.

```tsx
// BAD: renders "0" on screen
{count && <List />}

// GOOD
{count > 0 ? <List /> : null}
```

### Hydration flicker prevention

For client-only data (theme, locale), inject a sync `<script>` to set DOM before first paint.

```tsx
<script dangerouslySetInnerHTML={{ __html: `
  document.documentElement.dataset.theme =
    localStorage.getItem('theme') || 'light'
` }} />
```

### CSS content-visibility for long lists

```css
.message-row {
  content-visibility: auto;
  contain-intrinsic-size: auto 80px;
}
```

Skips layout/paint for off-screen items. Up to 10x faster initial render for long lists.

## 5. Bundle Size

### Barrel file imports

Import from source files, not barrel re-exports. Or configure `optimizePackageImports` in Next.js 13.5+.

```tsx
// BAD: pulls entire icon library
import { Check } from 'lucide-react'

// GOOD
import Check from 'lucide-react/dist/esm/icons/check'

// ALSO GOOD: next.config.js
{ experimental: { optimizePackageImports: ['lucide-react'] } }
```

### Preload on user intent

Preload heavy modules on hover/focus to reduce perceived latency.

```tsx
function NavItem({ href }: { href: string }) {
  const preload = () => { import('./heavy-page') }
  return <a href={href} onMouseEnter={preload} onFocus={preload}>Go</a>
}
```

## 6. JavaScript Patterns (for hot paths only)

### Use .toSorted() / .toReversed() — never .sort() on state/props

`.sort()` mutates in place. In React, this silently corrupts state.

```tsx
// BAD: mutates props
const sorted = users.sort((a, b) => a.name.localeCompare(b.name))

// GOOD
const sorted = users.toSorted((a, b) => a.name.localeCompare(b.name))
```

Also available: `.toReversed()`, `.toSpliced()`, `.with()`.

### Build a Map for repeated .find() lookups

```tsx
// BAD: O(n) per order
orders.map(o => ({ ...o, user: users.find(u => u.id === o.userId) }))

// GOOD: O(1) per order
const userMap = new Map(users.map(u => [u.id, u]))
orders.map(o => ({ ...o, user: userMap.get(o.userId) }))
```

## 7. Advanced

### Stable event handlers in custom hooks

Store callbacks in refs to prevent effect re-subscription.

```tsx
function useInterval(callback: () => void, ms: number) {
  const ref = useRef(callback)
  useEffect(() => { ref.current = callback })
  useEffect(() => {
    const id = setInterval(() => ref.current(), ms)
    return () => clearInterval(id)
  }, [ms])
}
```

When `useEffectEvent` becomes stable, prefer that instead.

### One-time app initialization

Don't rely on `useEffect([], ...)` for app-wide init. Components remount in StrictMode.

```tsx
let didInit = false
function App() {
  useEffect(() => {
    if (didInit) return
    didInit = true
    loadFromStorage()
    checkAuth()
  }, [])
}
```
