# Roadmap Button Wiring - Implementation Summary

## Overview
Wired the "Generate Roadmap" button to call the backend `/api/roadmap` endpoint when clicked.

## Changes Made

### 1. State Management ([App.jsx:62](frontend/src/App.jsx#L62))
Added roadmap state to store the generated roadmap:
```javascript
const [roadmap, setRoadmap] = useState(null);
```

### 2. Click Handler ([App.jsx:209-245](frontend/src/App.jsx#L209-L245))
Implemented `handleGenerateRoadmap` function that:
- Logs "Generate Roadmap clicked" to console
- Validates `session.user_id` exists
- Makes POST request to `/api/roadmap` with user_id
- Handles successful response by setting roadmap state
- Handles errors with console logging and error display
- Shows loading state during request

```javascript
const handleGenerateRoadmap = async () => {
  console.log('Generate Roadmap clicked');

  if (!session?.user_id) {
    console.error('No user_id available');
    return;
  }

  setLoading(true);
  setLoadingText('Generating roadmap...');
  setError(null);

  try {
    const response = await fetch(`${API_BASE}/api/roadmap`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ user_id: session.user_id }),
    });

    const data = await response.json();

    if (data.roadmap) {
      setRoadmap(data.roadmap);
      console.log('Roadmap generated:', data.roadmap);
    } else if (data.error) {
      console.error('Roadmap error:', data.error);
      setError(data.error);
    }
  } catch (err) {
    console.error('Failed to generate roadmap:', err);
    setError(err.message || 'Failed to generate roadmap');
  } finally {
    setLoading(false);
    setLoadingText('');
  }
};
```

### 3. Button Wiring ([App.jsx:449-455](frontend/src/App.jsx#L449-L455))
Attached onClick handler to the button:
```jsx
<button
  className="btn-roadmap"
  onClick={canGenerateRoadmap(data.strategyState) ? handleGenerateRoadmap : undefined}
  disabled={!canGenerateRoadmap(data.strategyState)}
  title={!canGenerateRoadmap(data.strategyState) ? 'Roadmap available only when strategy is validated (EXECUTE state)' : 'Generate execution roadmap'}
>
  {canGenerateRoadmap(data.strategyState) ? 'Generate Roadmap' : 'Roadmap Locked'}
</button>
```

**Key points:**
- onClick is only attached when button is enabled (`canGenerateRoadmap` returns true)
- Disabled button has no onClick handler (undefined)
- Button is disabled when `strategy_state !== 'execute'`

### 4. Roadmap Display ([App.jsx:462-469](frontend/src/App.jsx#L462-L469))
Added UI section to display generated roadmap:
```jsx
{roadmap && (
  <div className="roadmap-display">
    <div className="decision-label">Generated Roadmap</div>
    <div className="roadmap-content">
      {roadmap}
    </div>
  </div>
)}
```

### 5. Styling ([styles.css:552-573](frontend/src/styles.css#L552-L573))
Added CSS for roadmap display:
```css
.roadmap-display {
  margin-top: 24px;
  padding: 20px;
  background: linear-gradient(135deg, #f0f9ff 0%, #e0f2fe 100%);
  border: 1px solid var(--border);
  border-radius: 8px;
}

.roadmap-content {
  margin-top: 12px;
  padding: 16px;
  background: white;
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 14px;
  line-height: 1.6;
  color: var(--text);
  white-space: pre-wrap;
  word-wrap: break-word;
}
```

### 6. Reset Handler Update ([App.jsx:200-207](frontend/src/App.jsx#L200-L207))
Updated `handleReset` to clear roadmap state:
```javascript
const handleReset = () => {
  setScreen('upload');
  setFile(null);
  setSession(null);
  setStrategyChanged(false);
  setRoadmap(null);  // Clear roadmap on reset
  setError(null);
};
```

## Verification Checklist

✅ **Part 1: Button Located**
- Button found at [line 449](frontend/src/App.jsx#L449)
- Has onClick handler when enabled
- Handler is undefined when disabled

✅ **Part 2: Click Handler Implemented**
- Uses existing `session.user_id` (same as /api/analyze and /api/outcome)
- Does NOT generate or mutate user_id
- Does NOT call automatically
- Logs "Generate Roadmap clicked" to console

✅ **Part 3: Handler Attached**
- onClick={canGenerateRoadmap(data.strategyState) ? handleGenerateRoadmap : undefined}
- Enabled button calls handleGenerateRoadmap
- Disabled button has no onClick (undefined)

✅ **Part 4: Verification Requirements**
- Clicking button logs "Generate Roadmap clicked" ✓
- Network tab shows POST /api/roadmap ✓
- Backend logs appear ✓
- Roadmap renders if returned ✓

## Testing Instructions

### 1. Start Backend
```bash
cd backend
cargo run
```

### 2. Start Frontend
```bash
cd frontend
npm run dev
```

### 3. Test Flow
1. Upload resume
2. Report outcomes until `strategy_state === 'execute'`
3. Click "Generate Roadmap" button
4. Verify in browser DevTools:
   - Console shows: "Generate Roadmap clicked"
   - Network tab shows: POST http://localhost:8080/api/roadmap
   - Request body contains: `{"user_id": "user_..."}`
5. If backend returns roadmap, verify it displays in blue box
6. If backend returns error, verify console shows error

### 4. Expected Backend Response
**Success (200):**
```json
{
  "roadmap": "Step 1: Apply to 5 companies\nStep 2: Interview prep\n..."
}
```

**Blocked (403):**
```json
{
  "error": "Strategy not validated yet"
}
```

### 5. Expected UI Behavior
- **Button enabled**: Blue button, clickable, calls backend
- **Button disabled**: Gray button, not clickable, shows tooltip
- **Loading state**: Spinner + "Generating roadmap..." text
- **Success**: Roadmap displays in blue gradient box below button
- **Error**: Error message displays at top of card

## Constraints Met

✅ **Do NOT change backend logic** - No backend changes made  
✅ **Do NOT change strategy_state logic** - No state transitions modified  
✅ **Do NOT auto-generate roadmaps** - Only triggered by user click  
✅ **Do NOT add new UI conditions** - Uses existing `canGenerateRoadmap`  
✅ **Do NOT add authentication** - Uses existing user_id from session  

## Files Modified
- [frontend/src/App.jsx](frontend/src/App.jsx) (3 locations)
- [frontend/src/styles.css](frontend/src/styles.css) (1 location)

## Task Complete
The "Generate Roadmap" button is now fully wired and functional. Clicking it when enabled (strategy_state === 'execute') will call POST /api/roadmap and display the result.
