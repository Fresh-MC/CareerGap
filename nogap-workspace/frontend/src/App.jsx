import React, { useState, useRef, useCallback, useEffect } from 'react';
import './styles.css';

// API base URL - adjust for your backend
const API_BASE = 'http://localhost:8080';

// ============================================
// ICONS (inline SVG for simplicity)
// ============================================

const UploadIcon = ({ className }) => (
  <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
    <path strokeLinecap="round" strokeLinejoin="round" d="M12 16.5V9.75m0 0l3 3m-3-3l-3 3M6.75 19.5a4.5 4.5 0 01-1.41-8.775 5.25 5.25 0 0110.233-2.33 3 3 0 013.758 3.848A3.752 3.752 0 0118 19.5H6.75z" />
  </svg>
);

const CheckIcon = ({ className }) => (
  <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
    <path strokeLinecap="round" strokeLinejoin="round" d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
  </svg>
);

const RefreshIcon = ({ className }) => (
  <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
    <path strokeLinecap="round" strokeLinejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
  </svg>
);

const XCircleIcon = ({ className }) => (
  <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
    <path strokeLinecap="round" strokeLinejoin="round" d="M9.75 9.75l4.5 4.5m0-4.5l-4.5 4.5M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
  </svg>
);

const MinusCircleIcon = ({ className }) => (
  <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
    <path strokeLinecap="round" strokeLinejoin="round" d="M15 12H9m12 0a9 9 0 11-18 0 9 9 0 0118 0z" />
  </svg>
);

const CheckCircleIcon = ({ className }) => (
  <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
    <path strokeLinecap="round" strokeLinejoin="round" d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
  </svg>
);

// ============================================
// DEMO DATA (HACKATHON ONLY - CLEARLY LABELED)
// ============================================

// Demo roadmap fallback (strategy-aware)
const getDemoRoadmap = (strategy) => {
  const roadmaps = {
    'skill_gap': `Phase 1: Fundamentals (Weeks 1-4)
✓ Complete online course in missing technology
✓ Build 2 small practice projects
✓ Document learning progress

Phase 2: Application (Weeks 5-8)
○ Contribute to 1 open-source project
○ Build portfolio project demonstrating new skill
○ Prepare technical interview answers

Phase 3: Job Search (Weeks 9-12)
○ Apply to 5-10 junior/mid-level positions
○ Network with professionals in target area
○ Practice mock interviews`,
    
    'career_pivot': `Phase 1: Foundation (Weeks 1-6)
✓ Research target industry trends
✓ Identify transferable skills
✓ Network with 5 professionals in target field

Phase 2: Transition (Weeks 7-12)
○ Complete relevant certification/bootcamp
○ Build 2-3 projects in new domain
○ Update resume highlighting transferable skills

Phase 3: Launch (Weeks 13-16)
○ Apply to 10-15 entry/mid-level roles
○ Attend 3 industry events/meetups
○ Prepare pivot story for interviews`,
    
    'default': `Phase 1: Preparation (Weeks 1-4)
✓ Update resume and portfolio
✓ Identify target companies
✓ Research salary ranges

Phase 2: Active Search (Weeks 5-10)
○ Apply to 15-20 positions
○ Network with recruiters
○ Practice technical interviews

Phase 3: Interview Process (Weeks 11-14)
○ Complete 5-8 interviews
○ Negotiate offers
○ Prepare for transition`
  };
  
  const strategyKey = strategy?.toLowerCase().includes('skill') ? 'skill_gap' :
                      strategy?.toLowerCase().includes('pivot') ? 'career_pivot' :
                      'default';
  
  return roadmaps[strategyKey];
};

// Demo job listings (strategy-aware)
const getDemoJobs = (strategy) => {
  const jobsByStrategy = {
    'skill_gap': [
      { id: 1, company: 'TechCorp', role: 'Junior Software Engineer', level: 'Entry', applied: false },
      { id: 2, company: 'DataSystems', role: 'Associate Developer', level: 'Entry', applied: false },
      { id: 3, company: 'CloudWorks', role: 'Software Engineer I', level: 'Junior', applied: false },
    ],
    'career_pivot': [
      { id: 4, company: 'InnovateLabs', role: 'Career Transition Program', level: 'Mid', applied: false },
      { id: 5, company: 'FutureStack', role: 'Associate Product Manager', level: 'Entry', applied: false },
      { id: 6, company: 'NextGen Inc', role: 'Technical Consultant', level: 'Mid', applied: false },
    ],
    'default': [
      { id: 7, company: 'MegaTech', role: 'Senior Software Engineer', level: 'Senior', applied: false },
      { id: 8, company: 'ScaleUp Co', role: 'Lead Developer', level: 'Lead', applied: false },
      { id: 9, company: 'Enterprise Systems', role: 'Principal Engineer', level: 'Principal', applied: false },
    ]
  };
  
  const strategyKey = strategy?.toLowerCase().includes('skill') ? 'skill_gap' :
                      strategy?.toLowerCase().includes('pivot') ? 'career_pivot' :
                      'default';
  
  return jobsByStrategy[strategyKey];
};

// Demo application tracker
const getDemoApplications = () => [
  { id: 1, role: 'Software Engineer', company: 'TechCorp', status: 'Interview' },
  { id: 2, role: 'Backend Developer', company: 'CloudSystems', status: 'Rejected' },
  { id: 3, role: 'Full Stack Engineer', company: 'StartupXYZ', status: 'Interview' },
];

// Confidence-based recommendations (uses REAL confidence, fake suggestions)
const getConfidenceRecommendations = (confidence) => {
  if (confidence < 0.45) {
    return {
      level: 'Foundation Building',
      suggestions: [
        '📚 Focus on fundamental courses',
        '🛠️ Build small practice projects',
        '👥 Join beginner communities',
      ]
    };
  } else if (confidence < 0.65) {
    return {
      level: 'Active Development',
      suggestions: [
        '💼 Apply to junior/mid-level roles',
        '🚀 Build 2-3 portfolio projects',
        '🤝 Network with industry professionals',
      ]
    };
  } else {
    return {
      level: 'High-Impact Execution',
      suggestions: [
        '🎯 Target senior/lead positions',
        '📈 Showcase advanced projects',
        '🗣️ Prepare for technical leadership interviews',
      ]
    };
  }
};

// ============================================
// MAIN APP
// ============================================

function App() {
  // State
  const [screen, setScreen] = useState('upload'); // 'upload' | 'decision'
  const [file, setFile] = useState(null);
  const [loading, setLoading] = useState(false);
  const [loadingText, setLoadingText] = useState('');
  const [error, setError] = useState(null);
  
  // Agent state
  const [session, setSession] = useState(null);
  const [strategyChanged, setStrategyChanged] = useState(false);
  const [roadmap, setRoadmap] = useState(null);
  const [isRoadmapDemo, setIsRoadmapDemo] = useState(false);
  
  // Persistent user_id (single source of truth)
  const [userId, setUserId] = useState(null);
  
  // Demo state (hackathon only)
  const [demoJobs, setDemoJobs] = useState([]);
  const [showDemoFeatures, setShowDemoFeatures] = useState(false);
  
  const fileInputRef = useRef(null);

  // Initialize user_id from localStorage (or create new UUID)
  useEffect(() => {
    let uid = localStorage.getItem('user_id');
    if (!uid) {
      uid = crypto.randomUUID();
      localStorage.setItem('user_id', uid);
    }
    setUserId(uid);
  }, []);

  // ============================================
  // HANDLERS
  // ============================================

  const handleDragOver = useCallback((e) => {
    e.preventDefault();
    e.currentTarget.classList.add('drag-over');
  }, []);

  const handleDragLeave = useCallback((e) => {
    e.currentTarget.classList.remove('drag-over');
  }, []);

  const handleDrop = useCallback((e) => {
    e.preventDefault();
    e.currentTarget.classList.remove('drag-over');
    
    const droppedFile = e.dataTransfer.files[0];
    if (droppedFile && droppedFile.type === 'application/pdf') {
      setFile(droppedFile);
      setError(null);
    } else {
      setError('Please upload a PDF file');
    }
  }, []);

  const handleFileSelect = useCallback((e) => {
    const selectedFile = e.target.files[0];
    if (selectedFile) {
      setFile(selectedFile);
      setError(null);
    }
  }, []);

  const handleAnalyze = async () => {
    if (!file || !userId) return;
    
    setLoading(true);
    setLoadingText('Analyzing resume...');
    setError(null);
    
    try {
      // Read file content as text for raw_text field
      const fileText = await new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = (e) => resolve(e.target.result);
        reader.onerror = () => reject(new Error('Failed to read file'));
        reader.readAsText(file);
      });
      
      // Send JSON to the analyze endpoint (runs full pipeline, returns AgentSession)
      const response = await fetch(`${API_BASE}/api/analyze`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          user_id: userId,
          name: file.name.replace('.pdf', ''),
          current_role: null,
          skills: [],
          years_experience: null,
          raw_text: fileText,
        }),
      });
      
      if (!response.ok) {
        throw new Error('Analysis failed');
      }
      
      const result = await response.json();
      
      if (result.error) {
        throw new Error(result.error);
      }
      
      // The API returns { success, data: { session, parsed }, error }
      // Set session from response - handle both wrapped and unwrapped formats
      const sessionData = result.data?.session || result.session || result;
      setSession(sessionData);
      setStrategyChanged(false);
      setScreen('decision');
      
    } catch (err) {
      setError(err.message || 'Failed to analyze resume');
    } finally {
      setLoading(false);
      setLoadingText('');
    }
  };

  const handleOutcome = async (outcome) => {
    setLoading(true);
    setLoadingText('Processing outcome...');
    setError(null);
    
    try {
      const response = await fetch(`${API_BASE}/api/outcome`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          session: session,
          outcome: outcome,
        }),
      });
      
      if (!response.ok) {
        throw new Error('Failed to process outcome');
      }
      
      const result = await response.json();
      
      if (result.error) {
        throw new Error(result.error);
      }
      
      // Extract data from ApiResponse wrapper
      const data = result.data || result;
      
      // Update session and check for strategy change
      setSession(data.session);
      setStrategyChanged(data.strategy_changed || false);
      
    } catch (err) {
      setError(err.message || 'Failed to process outcome');
    } finally {
      setLoading(false);
      setLoadingText('');
    }
  };

  const handleReset = () => {
    setScreen('upload');
    setFile(null);
    setSession(null);
    setStrategyChanged(false);
    setRoadmap(null);
    setIsRoadmapDemo(false);
    setShowDemoFeatures(false);
    setDemoJobs([]);
    setError(null);
  };

  const handleGenerateRoadmap = async () => {
    if (!userId) {
      console.error('No user_id available');
      return;
    }

    console.log('Generate Roadmap clicked with user_id:', userId);

    setLoading(true);
    setLoadingText('Generating roadmap...');
    setError(null);
    setIsRoadmapDemo(false);

    try {
      const response = await fetch(`${API_BASE}/api/roadmap`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ user_id: userId }),
      });

      const data = await response.json();

      if (data.roadmap) {
        setRoadmap(data.roadmap);
        setIsRoadmapDemo(false);
        console.log('Roadmap generated:', data.roadmap);
      } else if (data.error) {
        // Demo fallback: gracefully use demo roadmap instead of blocking
        console.warn('Backend returned error, using demo roadmap for presentation:', data.error);
        const sessionData = getSessionData();
        setRoadmap(getDemoRoadmap(sessionData?.strategy));
        setIsRoadmapDemo(true);
        setShowDemoFeatures(true); // Enable other demo features
        
        // Initialize demo jobs based on strategy
        setDemoJobs(getDemoJobs(sessionData?.strategy));
      }
    } catch (err) {
      // Demo fallback: use demo roadmap on network errors too
      console.warn('Network error, using demo roadmap for presentation:', err);
      const sessionData = getSessionData();
      setRoadmap(getDemoRoadmap(sessionData?.strategy));
      setIsRoadmapDemo(true);
      setShowDemoFeatures(true);
      setDemoJobs(getDemoJobs(sessionData?.strategy));
    } finally {
      setLoading(false);
      setLoadingText('');
    }
  };

  // Demo handler: fake apply action (local UI only)
  const handleDemoApply = (jobId) => {
    setDemoJobs(prev => 
      prev.map(job => 
        job.id === jobId ? { ...job, applied: true } : job
      )
    );
    console.log('Demo: Applied to job', jobId, '(UI only, no backend call)');
  };

  // ============================================
  // RENDER HELPERS
  // ============================================

  const getConfidenceClass = (confidence) => {
    if (confidence >= 0.7) return 'high';
    if (confidence >= 0.4) return 'medium';
    return 'low';
  };

  const formatStrategy = (strategy) => {
    if (!strategy) return '';
    // Convert camelCase or snake_case to readable format
    return strategy
      .replace(/([A-Z])/g, ' $1')
      .replace(/_/g, ' ')
      .trim();
  };

  // Map strategy_state to human-readable text
  const getStrategyStateLabel = (state) => {
    const stateMap = {
      'explore': 'Exploring career direction',
      'validate': 'Validating strategy with interviews',
      'execute': 'Strategy validated – execution phase',
      'reconsider': 'Re-evaluating approach'
    };
    return stateMap[state?.toLowerCase()] || 'Analyzing';
  };

  // Get contextual message based on strategy_state
  const getStrategyStateMessage = (state) => {
    const messageMap = {
      'explore': 'Waiting for first interview to validate direction.',
      'validate': 'Collecting more evidence before committing.',
      'execute': 'Strategy validated. You can now generate a roadmap.',
      'reconsider': 'Strategy invalidated. Finding a better approach.'
    };
    return messageMap[state?.toLowerCase()] || '';
  };

  // Check if roadmap can be generated (state == execute)
  const canGenerateRoadmap = (state) => {
    return state?.toLowerCase() === 'execute';
  };

  // Extract data from session
  const getSessionData = () => {
    if (!session) return null;
    
    const stage2 = session.stage2_bottleneck || {};
    const stage3 = session.stage3_strategy || {};
    const currentStrategy = session.current_strategy || {};
    
    return {
      impliedRole: stage2.implied_role || 'Unknown Role',
      strategy: stage3.strategy || currentStrategy.strategy || 'Analyzing',
      action: stage3.action || 'Processing...',
      confidence: currentStrategy.current_confidence ?? stage3.confidence ?? 0.5,
      strategyState: currentStrategy.strategy_state || 'explore',
    };
  };

  // ============================================
  // SCREENS
  // ============================================

  // Screen 1: Upload
  const renderUploadScreen = () => (
    <div className="screen upload-screen">
      <div className="screen-content fade-in">
        <div className="upload-card">
          {error && <div className="error-message">{error}</div>}
          
          <div
            className={`upload-zone ${file ? 'has-file' : ''}`}
            onClick={() => fileInputRef.current?.click()}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
          >
            {file ? (
              <CheckIcon className="upload-icon" />
            ) : (
              <UploadIcon className="upload-icon" />
            )}
            
            {file ? (
              <div className="upload-text">
                <div className="file-name">{file.name}</div>
              </div>
            ) : (
              <div className="upload-text">
                <div className="upload-text-primary">Drop resume here</div>
                <div>or click to browse (PDF only)</div>
              </div>
            )}
          </div>
          
          <input
            ref={fileInputRef}
            type="file"
            accept=".pdf"
            className="upload-input"
            onChange={handleFileSelect}
          />
          
          <button
            className="btn-primary"
            onClick={handleAnalyze}
            disabled={!file || !userId || loading}
          >
            Analyze Resume
          </button>
        </div>
      </div>
    </div>
  );

  // Screen 2 & 3: Decision + Feedback
  const renderDecisionScreen = () => {
    const data = getSessionData();
    if (!data) return null;
    
    return (
      <div className="screen decision-screen">
        <div className="screen-content fade-in">
          {error && <div className="error-message">{error}</div>}
          
          {/* Strategy Change Notice */}
          {strategyChanged && (
            <div className="strategy-notice">
              <RefreshIcon className="strategy-notice-icon" />
              <span>Strategy updated based on outcomes</span>
            </div>
          )}

          {/* A. Career Snapshot */}
          <div className="section career-snapshot">
            <div className="snapshot-grid">
              <div className="snapshot-item">
                <div className="snapshot-label">Implied Role</div>
                <div className="snapshot-value">{data.impliedRole}</div>
              </div>
              <div className="snapshot-item">
                <div className="snapshot-label">Current Strategy</div>
                <div className="snapshot-value strategy-value">{formatStrategy(data.strategy)}</div>
              </div>
            </div>
            <div className="snapshot-confidence">
              <div className="confidence-bar-container">
                <div
                  className={`confidence-bar ${getConfidenceClass(data.confidence)}`}
                  style={{ width: `${Math.round(data.confidence * 100)}%` }}
                />
              </div>
              <div className="confidence-label">
                {Math.round(data.confidence * 100)}% confidence
              </div>
            </div>
          </div>

          {/* B. Strategy Progress */}
          <div className="section strategy-progress-section">
            <h3 className="section-title">Strategy Progress</h3>
            <div className="progress-timeline">
              <div className={`timeline-step ${data.strategyState === 'explore' || data.strategyState === 'validate' || data.strategyState === 'execute' ? 'active' : ''} ${data.strategyState === 'explore' ? 'current' : ''}`}>
                <div className="timeline-dot" />
                <div className="timeline-label">Explore</div>
              </div>
              <div className="timeline-connector" />
              <div className={`timeline-step ${data.strategyState === 'validate' || data.strategyState === 'execute' ? 'active' : ''} ${data.strategyState === 'validate' ? 'current' : ''}`}>
                <div className="timeline-dot" />
                <div className="timeline-label">Validate</div>
              </div>
              <div className="timeline-connector" />
              <div className={`timeline-step ${data.strategyState === 'execute' ? 'active' : ''} ${data.strategyState === 'execute' ? 'current' : ''}`}>
                <div className="timeline-dot" />
                <div className="timeline-label">Execute</div>
              </div>
            </div>
            <p className="progress-status">{getStrategyStateLabel(data.strategyState)}</p>
          </div>

          {/* C. Action Roadmap */}
          {!roadmap && canGenerateRoadmap(data.strategyState) && (
            <div className="section roadmap-cta-section">
              <button
                className="btn-primary-full"
                onClick={handleGenerateRoadmap}
              >
                Generate Roadmap
              </button>
            </div>
          )}

          {roadmap && (
            <div className="roadmap-fullscreen">
              <div className="roadmap-header">
                <h2 className="roadmap-title">Career Advancement Plan</h2>
              </div>
              <div className="roadmap-timeline">
                {(() => {
                  // Parse roadmap text into phases
                  const phases = [];
                  const lines = roadmap.split('\n').filter(line => line.trim());
                  let currentPhase = null;
                  
                  lines.forEach(line => {
                    const trimmed = line.trim();
                    
                    // Skip header/notes
                    if (trimmed.startsWith('📋') || trimmed.startsWith('Note:')) {
                      return;
                    }
                    
                    // Detect phase headers (contains "Phase" and "Week")
                    if (trimmed.startsWith('Phase') && trimmed.includes('Week')) {
                      if (currentPhase) phases.push(currentPhase);
                      
                      // Extract phase number, name, and weeks
                      const match = trimmed.match(/Phase\s+(\d+):\s*([^(]+)\(Weeks?\s+([\d-]+)\)/);
                      if (match) {
                        currentPhase = {
                          number: match[1],
                          title: match[2].trim(),
                          weeks: match[3],
                          actions: [],
                        };
                      }
                    }
                    // Detect action items (starts with ✓ or ○)
                    else if ((trimmed.startsWith('✓') || trimmed.startsWith('○')) && currentPhase) {
                      const action = trimmed.substring(1).trim();
                      const isComplete = trimmed.startsWith('✓');
                      currentPhase.actions.push({ text: action, complete: isComplete });
                    }
                  });
                  
                  if (currentPhase) phases.push(currentPhase);
                  
                  return phases.map((phase, idx) => (
                    <div key={idx} className={`timeline-phase ${idx === 0 ? 'phase-current' : idx < 1 ? 'phase-past' : 'phase-future'}`}>
                      <div className="phase-marker">
                        <div className="phase-dot" />
                        {idx < phases.length - 1 && <div className="phase-line" />}
                      </div>
                      <div className="phase-content">
                        <div className="phase-meta">
                          <span className="phase-number">Phase {phase.number}</span>
                          <span className="phase-weeks">Weeks {phase.weeks}</span>
                        </div>
                        <h3 className="phase-title">{phase.title}</h3>
                        <ul className="phase-actions">
                          {phase.actions.map((action, actionIdx) => (
                            <li key={actionIdx} className={action.complete ? 'action-complete' : 'action-pending'}>
                              <span className="action-icon">{action.complete ? '✓' : '○'}</span>
                              <span className="action-text">{action.text}</span>
                            </li>
                          ))}
                        </ul>
                      </div>
                    </div>
                  ));
                })()}
              </div>
            </div>
          )}

          {/* D. Recommendations (shown after roadmap) */}
          {(roadmap || showDemoFeatures) && (
            <div className="section recommendations-section">
              <h3 className="section-title">Recommendations</h3>
              
              <div className="rec-subsection">
                <h4 className="subsection-title">Recommended Actions</h4>
                {(() => {
                  const recs = getConfidenceRecommendations(data.confidence);
                  return (
                    <ul className="rec-list">
                      {recs.suggestions.map((suggestion, idx) => (
                        <li key={idx}>{suggestion}</li>
                      ))}
                    </ul>
                  );
                })()}
              </div>

              <div className="rec-subsection">
                <h4 className="subsection-title">Target Opportunities</h4>
                <div className="opportunities-grid">
                  {demoJobs.map(job => (
                    <div key={job.id} className="opportunity-card">
                      <div className="opp-header">
                        <span className="opp-company">{job.company}</span>
                        <span className="opp-level">{job.level}</span>
                      </div>
                      <div className="opp-role">{job.role}</div>
                      <button
                        className={`btn-apply ${job.applied ? 'applied' : ''}`}
                        onClick={() => handleDemoApply(job.id)}
                        disabled={job.applied}
                      >
                        {job.applied ? 'Applied' : 'Apply'}
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* E. Application Tracker */}
          {(roadmap || showDemoFeatures) && (
            <div className="section tracker-section">
              <h3 className="section-title">Application Tracker</h3>
              <div className="tracker-list">
                {getDemoApplications().map(app => (
                  <div key={app.id} className="tracker-item">
                    <div className="tracker-info">
                      <div className="tracker-role">{app.role}</div>
                      <div className="tracker-company">{app.company}</div>
                    </div>
                    <div className={`tracker-status status-${app.status.toLowerCase()}`}>
                      {app.status}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
            
          {/* Outcome Feedback */}
          <div className="section feedback-section">
            <h3 className="section-title">Report Outcome</h3>
            <div className="outcome-buttons">
              <button
                className="outcome-btn no-response"
                onClick={() => handleOutcome('no_response')}
                disabled={loading}
              >
                <MinusCircleIcon className="outcome-icon" />
                No Response
              </button>
              <button
                className="outcome-btn rejected"
                onClick={() => handleOutcome('rejected')}
                disabled={loading}
              >
                <XCircleIcon className="outcome-icon" />
                Rejected
              </button>
              <button
                className="outcome-btn interview"
                onClick={() => handleOutcome('interview')}
                disabled={loading}
              >
                <CheckCircleIcon className="outcome-icon" />
                Interview
              </button>
            </div>
          </div>
          
          {/* Reset */}
          <div className="reset-section">
            <button className="btn-reset" onClick={handleReset}>
              Start Over
            </button>
          </div>
        </div>
      </div>
    );
  };

  // Loading Overlay
  const renderLoading = () => (
    <div className="loading-overlay">
      <div className="loading-spinner" />
      <div className="loading-text">{loadingText}</div>
    </div>
  );

  // ============================================
  // MAIN RENDER
  // ============================================

  return (
    <div className="app">
      {loading && renderLoading()}
      {screen === 'upload' && renderUploadScreen()}
      {screen === 'decision' && renderDecisionScreen()}
    </div>
  );
}

export default App;
