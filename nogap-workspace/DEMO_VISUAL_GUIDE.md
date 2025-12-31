# Demo Visual Guide - What Judges Will See

## Demo Mode UI Preview

### 1. Demo Roadmap Display (When Backend Returns Error)

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ [DEMO] Generated Roadmap                  ┃
┃                                           ┃
┃ ⚠️  Demo roadmap for presentation        ┃
┃     purposes. Real roadmap requires       ┃
┃     backend session.                      ┃
┃                                           ┃
┃ ┌───────────────────────────────────────┐ ┃
┃ │ 📋 DEMO ROADMAP - Skill Gap Strategy │ ┃
┃ │                                       │ ┃
┃ │ Phase 1: Fundamentals (Weeks 1-4)    │ ┃
┃ │ ✓ Complete online course             │ ┃
┃ │ ✓ Build 2 small practice projects    │ ┃
┃ │ ✓ Document learning progress         │ ┃
┃ │                                       │ ┃
┃ │ Phase 2: Application (Weeks 5-8)     │ ┃
┃ │ ○ Contribute to open-source project  │ ┃
┃ │ ○ Build portfolio project            │ ┃
┃ │ ○ Prepare technical interviews       │ ┃
┃ │                                       │ ┃
┃ │ Phase 3: Job Search (Weeks 9-12)     │ ┃
┃ │ ○ Apply to 5-10 positions            │ ┃
┃ │ ○ Network with professionals         │ ┃
┃ │ ○ Practice mock interviews           │ ┃
┃ │                                       │ ┃
┃ │ Note: This is a demo roadmap for     │ ┃
┃ │ presentation purposes.                │ ┃
┃ └───────────────────────────────────────┘ ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
     ⬆️ Orange border indicates demo mode
```

---

### 2. Confidence-Based Recommendations

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ [DEMO] Recommendations Based on Confidence┃
┃                                           ┃
┃ ⚠️  Using real confidence value (72%)    ┃
┃     from backend                          ┃
┃                                           ┃
┃ High-Impact Execution                     ┃
┃                                           ┃
┃ ┌───────────────────────────────────────┐ ┃
┃ │ 🎯 Target senior/lead positions       │ ┃
┃ └───────────────────────────────────────┘ ┃
┃ ┌───────────────────────────────────────┐ ┃
┃ │ 📈 Showcase advanced projects         │ ┃
┃ └───────────────────────────────────────┘ ┃
┃ ┌───────────────────────────────────────┐ ┃
┃ │ 🗣️ Prepare for technical leadership  │ ┃
┃ │    interviews                          │ ┃
┃ └───────────────────────────────────────┘ ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
        ⬆️ Uses REAL confidence from backend
```

---

### 3. Sample Job Opportunities

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ [DEMO] Sample Opportunities               ┃
┃                                           ┃
┃ ⚠️  Static job listings for demo.        ┃
┃     Filtered by strategy: Skill Gap       ┃
┃                                           ┃
┃ ┌─────────────┐ ┌─────────────┐ ┌───────┐ ┃
┃ │ TechCorp    │ │ DataSystems │ │ Cloud │ ┃
┃ │     [Entry] │ │     [Entry] │ │Works  │ ┃
┃ │             │ │             │ │[Junior│ ┃
┃ │ Junior      │ │ Associate   │ │       │ ┃
┃ │ Software    │ │ Developer   │ │Softwa │ ┃
┃ │ Engineer    │ │             │ │re Eng │ ┃
┃ │             │ │             │ │I      │ ┃
┃ │  [Apply]    │ │  [Apply]    │ │[Apply]│ ┃
┃ └─────────────┘ └─────────────┘ └───────┘ ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
               ⬆️ Jobs match strategy
```

**After Clicking Apply:**
```
┌─────────────┐
│ TechCorp    │
│     [Entry] │
│             │
│ Junior      │
│ Software    │
│ Engineer    │
│             │
│ ✓ Applied   │ ← Green, disabled
└─────────────┘
```

---

### 4. Application Tracker

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ [DEMO] Application Tracker                ┃
┃                                           ┃
┃ ⚠️  User-reported outcomes (demo data)   ┃
┃                                           ┃
┃ ┌───────────────────────────────────────┐ ┃
┃ │ Software Engineer                     │ ┃
┃ │ TechCorp              [Interview] 🔵  │ ┃
┃ └───────────────────────────────────────┘ ┃
┃ ┌───────────────────────────────────────┐ ┃
┃ │ Backend Developer                     │ ┃
┃ │ CloudSystems          [Rejected]  🔴  │ ┃
┃ └───────────────────────────────────────┘ ┃
┃ ┌───────────────────────────────────────┐ ┃
┃ │ Full Stack Engineer                   │ ┃
┃ │ StartupXYZ            [Interview] 🔵  │ ┃
┃ └───────────────────────────────────────┘ ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
    ⬆️ Color-coded statuses (blue/red)
```

---

### 5. Roadmap Progress Tracking

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ [DEMO] Roadmap Progress                   ┃
┃                                           ┃
┃ ⚠️  Visual-only checkboxes               ┃
┃     (non-interactive, no persistence)     ┃
┃                                           ┃
┃ ┌───────────────────────────────────────┐ ┃
┃ │ ☑️  Phase 1: Foundation completed     │ ┃ ← Green bg
┃ └───────────────────────────────────────┘ ┃
┃ ┌───────────────────────────────────────┐ ┃
┃ │ ☐  Phase 2: Active development in    │ ┃
┃ │     progress                           │ ┃
┃ └───────────────────────────────────────┘ ┃
┃ ┌───────────────────────────────────────┐ ┃
┃ │ ☐  Phase 3: Execution pending        │ ┃
┃ └───────────────────────────────────────┘ ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
      ⬆️ Checkboxes are disabled (cursor: not-allowed)
```

---

## Complete Demo Screen Layout

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃                                                     ┃
┃  Implied Role: Senior Software Engineer             ┃
┃  Strategy: Skill Gap Closure                        ┃
┃  Current Action: Apply to companies matching skill  ┃
┃  Confidence: ████████████░░░░░░ 72% confidence     ┃
┃                                                     ┃
┃  Strategy Phase: Strategy validated – execution     ┃
┃                                                     ┃
┃  Explore ● ━━━━━ Validate ● ━━━━━ Execute ●       ┃
┃                                          ⬆️ active ┃
┃                                                     ┃
┃  ⚠️  Strategy validated. You can now generate a    ┃
┃      roadmap.                                       ┃
┃                                                     ┃
┃  [ Generate Roadmap ]  ← Enabled (blue button)     ┃
┃                                                     ┃
┃  ┌────────────────────────────────────────────┐   ┃
┃  │ [DEMO] Generated Roadmap                   │   ┃
┃  │ ⚠️  Demo roadmap for presentation...       │   ┃
┃  │ [Roadmap content...]                       │   ┃
┃  └────────────────────────────────────────────┘   ┃
┃                                                     ┃
┃  ┌────────────────────────────────────────────┐   ┃
┃  │ [DEMO] Recommendations Based on Confidence │   ┃
┃  │ ⚠️  Using real confidence (72%)...         │   ┃
┃  │ [Recommendations...]                       │   ┃
┃  └────────────────────────────────────────────┘   ┃
┃                                                     ┃
┃  ┌────────────────────────────────────────────┐   ┃
┃  │ [DEMO] Sample Opportunities                │   ┃
┃  │ ⚠️  Static job listings...                 │   ┃
┃  │ [Job cards with Apply buttons...]          │   ┃
┃  └────────────────────────────────────────────┘   ┃
┃                                                     ┃
┃  ┌────────────────────────────────────────────┐   ┃
┃  │ [DEMO] Application Tracker                 │   ┃
┃  │ ⚠️  User-reported outcomes (demo)...       │   ┃
┃  │ [Application list with statuses...]        │   ┃
┃  └────────────────────────────────────────────┘   ┃
┃                                                     ┃
┃  ┌────────────────────────────────────────────┐   ┃
┃  │ [DEMO] Roadmap Progress                    │   ┃
┃  │ ⚠️  Visual-only checkboxes...              │   ┃
┃  │ [Progress checklist...]                    │   ┃
┃  └────────────────────────────────────────────┘   ┃
┃                                                     ┃
┃  Report Outcome                                     ┃
┃  [ No Response ] [ Rejected ] [ Interview ]        ┃
┃                                                     ┃
┃  [ Start Over ]                                     ┃
┃                                                     ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

---

## Color Legend

🟡 **Orange (#f59e0b)** - Demo badges, borders on demo sections  
🟡 **Yellow (#fffbeb)** - Warning notices explaining demo features  
🔵 **Blue (#2563eb)** - Real backend data, active states, Interview status  
🟢 **Green (#10b981)** - Success (Applied buttons, completed progress)  
🔴 **Red (#ef4444)** - Rejected status  
⚪ **Gray (#e5e7eb)** - Disabled states, borders  

---

## Key Visual Indicators

### 1. DEMO Badge
```
[DEMO] ← Orange badge with white text, 11px, bold, 4px padding
```

### 2. Warning Notices
```
⚠️  Demo roadmap for presentation purposes.
    Real roadmap requires backend session.
    
    ⬆️ Yellow background (#fffbeb)
    ⬆️ Orange left border (3px)
    ⬆️ Dark yellow text (#92400e)
```

### 3. Border Styles
- **Real Roadmap:** Gray border, blue gradient background
- **Demo Roadmap:** Orange border (2px), yellow gradient background
- **Demo Sections:** Gray border, light gray background (#f9fafb)

### 4. Button States
```
Generate Roadmap (enabled):
  Background: Blue (#2563eb)
  Hover: Darker blue + lift effect
  
Generate Roadmap (disabled):
  Background: Gray (#e5e7eb)
  Cursor: not-allowed
  Tooltip: "Roadmap unlocks when strategy reaches EXECUTE"

Apply (normal):
  Background: Blue (#2563eb)
  Text: "Apply"
  
Apply (clicked):
  Background: Green (#10b981)
  Text: "✓ Applied"
  Disabled: true
```

---

## Judge Experience Flow

### Scenario A: Real Backend Works
```
1. Upload resume
2. Backend analyzes → Strategy + Confidence
3. Report outcomes → State transitions (EXPLORE → VALIDATE → EXECUTE)
4. Click "Generate Roadmap"
5. Backend returns real roadmap
6. See: Real roadmap (blue gradient, no DEMO badge)
7. No demo features shown
```

### Scenario B: Backend Returns Error (Demo Mode)
```
1. Upload resume
2. Backend analyzes → Strategy + Confidence
3. Report outcomes → State transitions (EXPLORE → VALIDATE → EXECUTE)
4. Click "Generate Roadmap"
5. Backend returns: { "error": "Resume not found" }
6. Frontend gracefully shows:
   ✓ Demo roadmap (orange border, DEMO badge)
   ✓ Recommendations (uses real confidence)
   ✓ Sample jobs (filtered by real strategy)
   ✓ Application tracker
   ✓ Progress tracking
7. All demo features clearly labeled
```

---

## Transparency Checklist

✅ **Every demo feature has a DEMO badge**  
✅ **Every demo section has an explanation notice**  
✅ **Demo roadmap has orange border (visual distinction)**  
✅ **Recommendations show "Using real confidence (X%)"**  
✅ **Jobs show "Static job listings for demo"**  
✅ **Tracker shows "User-reported outcomes (demo data)"**  
✅ **Progress shows "Visual-only, non-interactive, no persistence"**  
✅ **Console logs all demo actions**  

---

## What Judges Will Understand

### Immediately Clear:
1. **Demo features are clearly separated** from real logic
2. **Real backend strategy/confidence** drives demo content
3. **Strategy gating still enforced** (EXECUTE required)
4. **Presentation polish**, not architectural shortcuts
5. **Graceful error handling** prevents demo dead-ends

### Technical Integrity Preserved:
- Real state machine (EXPLORE → VALIDATE → EXECUTE → RECONSIDER)
- Real confidence calculations
- Real strategy selection
- Real API calls (demo only triggers on errors)
- No bypass of business rules

---

## Mobile/Responsive Behavior

On smaller screens, the layout adjusts:
- Job cards stack vertically
- Progress bars shrink appropriately
- Text remains readable
- DEMO badges stay visible
- Notices remain prominent

---

## Accessibility

All demo features include:
- Clear ARIA labels
- High contrast text
- Disabled states properly marked
- Console logging for screen readers
- Keyboard navigation support

---

## Summary

Judges will see a **polished, professional demo** with:
- ✅ Clear separation of demo vs real features
- ✅ Transparent labeling throughout
- ✅ Real backend logic preserved
- ✅ Graceful error handling
- ✅ Presentation-ready UI

The demo mode enhances the hackathon presentation without compromising the integrity of the agentic AI system.
