# Career Agent 🎯

**An Agentic AI Career Development Assistant**

Career Agent is a web-based tool that helps you plan and track your career development using an intelligent agent that learns and adapts over time.

## 🧠 Agentic Architecture

Career Agent implements a **Sense → Plan → Learn** loop:

```
┌─────────────────────────────────────────────────────────────┐
│                      SENSE                                   │
│  • Resume upload & skill extraction                          │
│  • Career goal setting                                       │
│  • Current state assessment                                  │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                       PLAN                                   │
│  • Goal-driven roadmap generation                            │
│  • Constraint evaluation (prerequisites, time)               │
│  • Human-in-the-loop editing                                 │
│  • Step prioritization and ordering                          │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                      LEARN                                   │
│  • Weekly reflection generation                              │
│  • Progress tracking                                         │
│  • Plan adaptation based on feedback                         │
│  • Memory timeline for full explainability                   │
└─────────────────────────────────────────────────────────────┘
```

## ✨ Features

### Agent Memory Timeline
Every action is recorded in a persistent timeline:
- Resume uploads
- Goal changes
- Roadmap generation
- Step completions
- Plan modifications
- Weekly reflections

### Weekly Reflection
The agent generates insights on your progress:
- What changed this week
- Why the plan adapted
- Suggestions for next steps

### Human-in-the-Loop Editing
Full control over your career roadmap:
- Edit step titles and descriptions
- Reorder steps
- Skip steps with reasons
- Add custom steps
- Remove steps

### Career Roadmap
Goal-driven planning with:
- Prerequisite tracking
- Time estimates
- Confidence scores
- Step explanations

## 🚀 Quick Start

### Prerequisites
- Rust (1.70+)
- Node.js (18+)
- npm or pnpm

### Run the Backend

```bash
cd nogap-workspace/backend
cargo run
```

The API server starts at `http://localhost:8080`

### Run the Frontend

```bash
cd nogap-workspace/frontend
npm install
npm run dev
```

The web UI is available at `http://localhost:3000`

## 📡 API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| POST | `/api/resume` | Upload resume data |
| POST | `/api/goal` | Set career goal |
| POST | `/api/roadmap` | Generate roadmap |
| GET | `/api/roadmap/:user_id` | Get current roadmap |
| POST | `/api/roadmap/edit` | Edit roadmap |
| POST | `/api/roadmap/:user_id/step/:step_id/complete` | Complete a step |
| GET | `/api/memory/:user_id` | Get memory timeline |
| GET | `/api/reflection/:user_id` | Get weekly reflection |
| GET | `/api/rules` | Get available career rules |

## 📁 Project Structure

```
nogap-workspace/
├── backend/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          # Entry point
│       ├── lib.rs           # Library exports
│       ├── api.rs           # Web API handlers
│       └── agent/
│           ├── mod.rs       # Agent module
│           ├── types.rs     # Career types (CareerRule, SkillAssessment, etc.)
│           ├── memory.rs    # Agent memory timeline
│           ├── planner.rs   # Career roadmap planner
│           └── reflection.rs # Weekly reflection generator
├── frontend/
│   ├── package.json
│   ├── vite.config.js
│   ├── index.html
│   └── src/
│       ├── main.jsx
│       ├── App.jsx
│       ├── styles.css
│       └── components/
│           ├── ResumeUpload.jsx
│           ├── GoalSetter.jsx
│           ├── RoadmapView.jsx
│           ├── MemoryTimeline.jsx
│           └── ReflectionCard.jsx
└── README.md
```

## 🔧 Core Concepts

### Career Types

| Old (Security) | New (Career) |
|----------------|--------------|
| Policy | CareerRule |
| Audit | SkillAssessment |
| Remediation | CareerAction |
| Snapshot | CareerCheckpoint |

### CareerRule
Defines a skill, milestone, or requirement:
```rust
CareerRule {
    id: "programming_fundamentals",
    title: "Programming Fundamentals",
    category: "technical_skill",
    priority: "critical",
    estimated_weeks: 4,
    prerequisites: [],
}
```

### Memory Event Types
- `resume_uploaded` - User uploaded resume
- `plan_generated` - Roadmap was generated
- `plan_modified` - Roadmap was edited
- `step_completed` - A step was marked complete
- `step_skipped` - A step was skipped
- `goal_set` - Career goal was set
- `reflection_generated` - Weekly reflection created

## 🛠️ Development

### Build Backend
```bash
cd backend
cargo build --release
```

### Build Frontend
```bash
cd frontend
npm run build
```

### Run Tests
```bash
cd backend
cargo test
```

## 📄 License

MIT License

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

---

**Career Agent** - Plan your career journey with AI assistance.
