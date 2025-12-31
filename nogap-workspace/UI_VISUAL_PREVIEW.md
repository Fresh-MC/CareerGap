# Strategy State UI - Visual Preview

## Component Overview

This document shows the visual appearance of the strategy state UI components in different states.

---

## 1. Strategy State Label

**Component**: Large, bold text showing human-readable state

### States:

#### EXPLORE
```
┌──────────────────────────────────┐
│ Strategy Phase                    │
│ Exploring career direction        │ ← Blue accent color
└──────────────────────────────────┘
```

#### VALIDATE
```
┌──────────────────────────────────┐
│ Strategy Phase                    │
│ Validating strategy with          │
│ interviews                        │
└──────────────────────────────────┘
```

#### EXECUTE
```
┌──────────────────────────────────┐
│ Strategy Phase                    │
│ Strategy validated – execution    │
│ phase                             │
└──────────────────────────────────┘
```

#### RECONSIDER
```
┌──────────────────────────────────┐
│ Strategy Phase                    │
│ Re-evaluating approach            │
└──────────────────────────────────┘
```

---

## 2. Progress Indicator

**Component**: Linear 3-step progress bar (Explore → Validate → Execute)

### Legend:
- `●` = Current step (large blue dot with glow)
- `○` = Active/passed step (blue outline)
- `○` = Inactive step (gray)
- `━` = Connection line

### State Visualizations:

#### EXPLORE (Initial)
```
● ━━━━ ○ ━━━━ ○
Explore   Validate   Execute
```
- Current: EXPLORE (bold blue, glowing)
- Next: VALIDATE (gray, inactive)
- Future: EXECUTE (gray, inactive)

#### VALIDATE (After 1st interview)
```
○ ━━━━ ● ━━━━ ○
Explore   Validate   Execute
```
- Passed: EXPLORE (blue outline)
- Current: VALIDATE (bold blue, glowing)
- Next: EXECUTE (gray, inactive)

#### EXECUTE (After 2nd interview)
```
○ ━━━━ ○ ━━━━ ●
Explore   Validate   Execute
```
- Passed: EXPLORE (blue outline)
- Passed: VALIDATE (blue outline)
- Current: EXECUTE (bold blue, glowing)

#### RECONSIDER (After failure)
```
○ ━━━━ ○ ━━━━ ○
Explore   Validate   Execute
```
- All steps inactive (gray)
- No current state highlighted
- User needs to restart

---

## 3. Contextual Messages

**Component**: Yellow info box with left border accent

### Message Box Style:
```
┌─────────────────────────────────────┐
│ ⚠                                   │
│ [Message text here]                 │
│                                     │
└─────────────────────────────────────┘
```
- Background: Light yellow (#fffbeb)
- Border-left: Orange (#f59e0b)
- Text: Dark brown (#78350f)

### Messages by State:

#### EXPLORE
```
┌─────────────────────────────────────┐
│ ⚠ Waiting for first interview to   │
│   validate direction.               │
└─────────────────────────────────────┘
```

#### VALIDATE
```
┌─────────────────────────────────────┐
│ ⚠ Collecting more evidence before  │
│   committing.                       │
└─────────────────────────────────────┘
```

#### EXECUTE
```
┌─────────────────────────────────────┐
│ ⚠ Strategy validated. You can now  │
│   generate a roadmap.               │
└─────────────────────────────────────┘
```

#### RECONSIDER
```
┌─────────────────────────────────────┐
│ ⚠ Strategy invalidated. Finding a  │
│   better approach.                  │
└─────────────────────────────────────┘
```

---

## 4. Roadmap Button

**Component**: Full-width button with conditional styling

### Button States:

#### EXPLORE / VALIDATE / RECONSIDER (Disabled)
```
┌─────────────────────────────────────┐
│                                     │
│      [  Roadmap Locked  ]          │ ← Gray, disabled
│                                     │
│  Roadmap unlocks when strategy      │ ← Tooltip text
│  reaches EXECUTE state              │
│                                     │
└─────────────────────────────────────┘
```
- Background: Light gray (#e5e7eb)
- Text: Muted gray (#9ca3af)
- Cursor: not-allowed
- No hover effect

#### EXECUTE (Enabled)
```
┌─────────────────────────────────────┐
│                                     │
│     [ Generate Roadmap ]           │ ← Blue, clickable
│                                     │
└─────────────────────────────────────┘
```
- Background: Accent blue (#2563eb)
- Text: White
- Cursor: pointer
- Hover: Lighter blue, slight lift

---

## 5. Complete Section Layout

### Full UI Section (EXECUTE State Example)

```
┌───────────────────────────────────────────────┐
│                                               │
│  Strategy Phase                               │
│  Strategy validated – execution phase         │ ← 18px, bold, blue
│                                               │
│  ● ━━━━ ○ ━━━━ ○                            │ ← Progress indicator
│  Explore  Validate  Execute                   │
│                                               │
│  ┌────────────────────────────────────────┐  │
│  │ ⚠ Strategy validated. You can now     │  │ ← Yellow info box
│  │   generate a roadmap.                  │  │
│  └────────────────────────────────────────┘  │
│                                               │
│  ┌────────────────────────────────────────┐  │
│  │      [ Generate Roadmap ]              │  │ ← Blue button
│  └────────────────────────────────────────┘  │
│                                               │
└───────────────────────────────────────────────┘
```

### Full UI Section (VALIDATE State Example)

```
┌───────────────────────────────────────────────┐
│                                               │
│  Strategy Phase                               │
│  Validating strategy with interviews          │
│                                               │
│  ○ ━━━━ ● ━━━━ ○                            │
│  Explore  Validate  Execute                   │
│                                               │
│  ┌────────────────────────────────────────┐  │
│  │ ⚠ Collecting more evidence before     │  │
│  │   committing.                          │  │
│  └────────────────────────────────────────┘  │
│                                               │
│  ┌────────────────────────────────────────┐  │
│  │      [  Roadmap Locked  ]              │  │ ← Gray button
│  └────────────────────────────────────────┘  │
│  Roadmap unlocks when strategy reaches        │
│  EXECUTE state                                │
│                                               │
└───────────────────────────────────────────────┘
```

---

## Design Specifications

### Colors

| Element | State | Color | Hex |
|---------|-------|-------|-----|
| State label | All | Accent blue | #2563eb |
| Progress dot | Inactive | Gray | #e5e7eb |
| Progress dot | Active | Light blue | #dbeafe |
| Progress dot | Current | Accent blue | #2563eb |
| Progress glow | Current | Blue (10% opacity) | rgba(37, 99, 235, 0.1) |
| Message box bg | All | Light yellow | #fffbeb |
| Message box border | All | Orange | #f59e0b |
| Message box text | All | Dark brown | #78350f |
| Button (enabled) | EXECUTE | Accent blue | #2563eb |
| Button (disabled) | Other states | Light gray | #e5e7eb |
| Button text (enabled) | EXECUTE | White | #ffffff |
| Button text (disabled) | Other states | Muted gray | #9ca3af |

### Typography

| Element | Size | Weight | Line Height |
|---------|------|--------|-------------|
| "Strategy Phase" label | 14px | 500 | 1.5 |
| State label | 18px | 600 | 1.3 |
| Progress labels | 12px | 500 (current: 600) | 1.2 |
| Message text | 14px | 400 | 1.5 |
| Button text | 15px | 600 | 1.4 |
| Tooltip text | 13px | 400 | 1.4 |

### Spacing

| Element | Margin/Padding |
|---------|----------------|
| Section container | padding: 20px |
| State label | margin-bottom: 20px |
| Progress indicator | margin: 24px 0 |
| Progress steps | gap: 8px (vertical) |
| Message box | margin-top: 16px, padding: 12px 16px |
| Roadmap section | margin-top: 24px |
| Button | padding: 14px 24px |
| Tooltip | margin-top: 8px |

### Dimensions

| Element | Size |
|---------|------|
| Progress dot (inactive) | 16px × 16px |
| Progress dot (current) | 20px × 20px |
| Progress line | height: 2px |
| Message box border-left | 3px |
| Button border-radius | 8px |

### Shadows

| Element | Shadow |
|---------|--------|
| Progress dot (current) | 0 0 0 4px rgba(37, 99, 235, 0.1) |
| Button (enabled) | 0 1px 2px rgba(0, 0, 0, 0.04) |
| Button (enabled, hover) | 0 4px 12px rgba(0, 0, 0, 0.06) |

---

## Interaction States

### Button Hover (EXECUTE state only)

**Normal**:
```css
background: #2563eb;
box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
transform: translateY(0);
```

**Hover**:
```css
background: #3b82f6; /* Lighter blue */
box-shadow: 0 4px 12px rgba(0, 0, 0, 0.06);
transform: translateY(-1px); /* Slight lift */
```

### Button Disabled (Other states)

**No hover effect**:
```css
cursor: not-allowed;
background: #e5e7eb; /* Always gray */
color: #9ca3af;
```

---

## Accessibility

### Semantic HTML
- Uses `<button>` for clickable elements
- Uses `disabled` attribute for unavailable actions
- Uses `title` attribute for tooltips

### ARIA Labels (Recommended additions)
```jsx
<div className="strategy-progress" role="progressbar" aria-valuenow={currentStep} aria-valuemin="1" aria-valuemax="3">
  {/* Progress steps */}
</div>

<button
  className="btn-roadmap"
  disabled={!canGenerateRoadmap(data.strategyState)}
  aria-label={canGenerateRoadmap(data.strategyState) ? 'Generate execution roadmap' : 'Roadmap locked until strategy validation'}
>
  {/* Button text */}
</button>
```

### Focus States
All interactive elements have default browser focus indicators (blue outline).

---

## Responsive Behavior

### Desktop (> 600px)
- Full width layout
- All elements visible

### Mobile (≤ 600px)
- Progress labels remain horizontal
- Button remains full width
- Message text wraps naturally
- No layout changes needed

---

## Animation

### None (Per Requirements)
- ❌ No transitions between states
- ❌ No progress bar animations
- ✅ Only CSS transitions on hover (button lift)

---

## Browser Compatibility

Tested/Expected to work on:
- Chrome/Edge (Chromium) 90+
- Firefox 88+
- Safari 14+
- Mobile browsers (iOS Safari, Chrome Android)

CSS Features Used:
- Flexbox ✅
- CSS Grid ❌ (not used)
- CSS Variables (custom properties) ✅
- Box shadows ✅
- Transforms (translateY) ✅
- Border-radius ✅

---

**Last Updated**: January 2026  
**Version**: 1.0  
**Status**: Production-ready
