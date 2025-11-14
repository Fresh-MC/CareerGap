import React, { useState, useEffect } from 'react';

// Tauri invoke function (will be available when running in Tauri)
declare global {
  interface Window {
    __TAURI__?: {
      invoke: (cmd: string, args?: any) => Promise<any>;
    };
  }
}

const App: React.FC = () => {
  const [version, setVersion] = useState<string>('Loading...');
  const [auditResult, setAuditResult] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    // Load version on mount
    loadVersion();
  }, []);

  const loadVersion = async () => {
    try {
      if (window.__TAURI__) {
        const ver = await window.__TAURI__.invoke('get_version');
        setVersion(ver);
      } else {
        setVersion('0.1.0 (Browser Mode)');
      }
    } catch (error) {
      console.error('Failed to get version:', error);
      setVersion('Error loading version');
    }
  };

  const runAudit = async () => {
    setIsLoading(true);
    setAuditResult('Running audit...');
    try {
      if (window.__TAURI__) {
        const result = await window.__TAURI__.invoke('run_audit');
        setAuditResult(result);
      } else {
        setAuditResult('NoGap Audit: System scan complete. No vulnerabilities detected. (Browser Mode)');
      }
    } catch (error) {
      console.error('Failed to run audit:', error);
      setAuditResult('Error running audit');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div style={{ padding: '20px', fontFamily: 'Arial, sans-serif' }}>
      <h1>NoGap Dashboard</h1>
      <p>Welcome to the NoGap Security Platform!</p>
      
      <div style={{ marginTop: '20px', padding: '15px', backgroundColor: '#f5f5f5', borderRadius: '5px' }}>
        <h2>System Information</h2>
        <p><strong>Version:</strong> {version}</p>
      </div>

      <div style={{ marginTop: '20px' }}>
        <button 
          onClick={runAudit}
          disabled={isLoading}
          style={{
            padding: '10px 20px',
            fontSize: '16px',
            cursor: isLoading ? 'not-allowed' : 'pointer',
            backgroundColor: isLoading ? '#ccc' : '#007bff',
            color: 'white',
            border: 'none',
            borderRadius: '5px'
          }}
        >
          {isLoading ? 'Running...' : 'Run Security Audit'}
        </button>
      </div>

      {auditResult && (
        <div style={{ 
          marginTop: '20px', 
          padding: '15px', 
          backgroundColor: '#e8f5e9', 
          borderRadius: '5px',
          border: '1px solid #4caf50'
        }}>
          <h3>Audit Results</h3>
          <p>{auditResult}</p>
        </div>
      )}

      <div style={{ marginTop: '30px', fontSize: '12px', color: '#666' }}>
        <p>
          <a href="/features" style={{ marginRight: '15px' }}>Features</a>
          <a href="/docs" style={{ marginRight: '15px' }}>CLI Documentation</a>
          <a href="/about">About</a>
        </p>
      </div>
    </div>
  );
};

export default App;