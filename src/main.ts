import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

// Types
interface CaptureStatus {
  is_capturing: boolean;
  frame_count: number;
  elapsed_seconds: number;
  average_fps: number;
}

interface TunnelInfo {
  id: string;
  subdomain: string;
  url: string;
  port: number;
  password: string | null;
  created_at: string;
}

interface User {
  email: string;
  id: string;
}

// Auth State
let currentUser: User | null = null;
let authToken: string | null = null;

// DOM Elements - Auth
const loginScreen = document.getElementById('login-screen')!;
const mainScreen = document.getElementById('main-screen')!;
const loginForm = document.getElementById('login-form') as HTMLFormElement;
const verifyForm = document.getElementById('verify-form') as HTMLFormElement;
const emailInput = document.getElementById('email') as HTMLInputElement;
const codeInput = document.getElementById('code') as HTMLInputElement;
const loginBtn = document.getElementById('login-btn') as HTMLButtonElement;
const verifyBtn = document.getElementById('verify-btn') as HTMLButtonElement;
const backBtn = document.getElementById('back-btn') as HTMLButtonElement;
const logoutBtn = document.getElementById('logout-btn') as HTMLButtonElement;
const authMessage = document.getElementById('auth-message')!;
const userEmailEl = document.getElementById('user-email')!;

// DOM Elements - Main App
const statusEl = document.getElementById('status')!;
const startBtn = document.getElementById('startCapture') as HTMLButtonElement;
const stopBtn = document.getElementById('stopCapture') as HTMLButtonElement;
const statsEl = document.getElementById('stats')!;
const connectTunnelBtn = document.getElementById('connectTunnel') as HTMLButtonElement;
const tunnelPasswordInput = document.getElementById('tunnel-password') as HTMLInputElement;
const togglePasswordBtn = document.getElementById('toggle-password') as HTMLButtonElement;
const tunnelInfoEl = document.getElementById('tunnelInfo')!;

// State
let statusInterval: number | null = null;
let pendingAuthId: string | null = null;

// ============================================================================
// AUTH FUNCTIONS
// ============================================================================

function showMessage(message: string, type: 'error' | 'success') {
  authMessage.innerHTML = `<div class="${type}">${message}</div>`;
  setTimeout(() => {
    authMessage.innerHTML = '';
  }, 5000);
}

function setLoading(btn: HTMLButtonElement, loading: boolean, text?: string) {
  btn.disabled = loading;
  if (loading) {
    btn.innerHTML = `<span class="spinner"></span>${text || 'Loading...'}`;
  } else {
    btn.textContent = text || btn.textContent!;
  }
}

async function sendMagicLink(email: string): Promise<string> {
  console.log('[WorkOS] Sending magic link to:', email);
  const authId = await invoke<string>('workos_send_magic_link', { email });
  console.log('[WorkOS] Auth ID received:', authId);
  return authId;
}

async function verifyMagicCode(code: string, authId: string): Promise<{ access_token: string; user: User }> {
  const result = await invoke<{ access_token: string; user: User }>('workos_verify_code', { code, authId });
  return result;
}

async function handleLogin(e: Event) {
  e.preventDefault();
  const email = emailInput.value.trim();

  if (!email) return;

  setLoading(loginBtn, true, 'Sending code...');

  try {
    pendingAuthId = await sendMagicLink(email);

    // Switch to verification form
    loginForm.classList.add('hidden');
    verifyForm.classList.remove('hidden');
    showMessage(`Code sent!`, 'success');
    codeInput.focus();
  } catch (error: any) {
    console.error('Login error:', error);
    const errorMsg = error.message || error.toString() || 'Failed to send code';
    showMessage(errorMsg, 'error');
  } finally {
    setLoading(loginBtn, false, 'Sign In');
  }
}

async function handleVerify(e: Event) {
  e.preventDefault();
  const code = codeInput.value.trim();

  console.log('[Auth] Verify attempt - code:', code, 'authId:', pendingAuthId);

  if (!code) {
    showMessage('Please enter the verification code', 'error');
    return;
  }

  if (!pendingAuthId) {
    showMessage('Missing authentication token. Please request a new code.', 'error');
    return;
  }

  setLoading(verifyBtn, true, 'Verifying...');

  try {
    const { access_token, user } = await verifyMagicCode(code, pendingAuthId);

    // Store auth state
    authToken = access_token;
    currentUser = user;

    // Save to localStorage
    localStorage.setItem('tnnl_token', access_token);
    localStorage.setItem('tnnl_user', JSON.stringify(user));

    // Show main screen
    showMainScreen();
  } catch (error: any) {
    console.error('[Auth] Verification error:', error);
    showMessage(error.message || 'Invalid code', 'error');
  } finally {
    setLoading(verifyBtn, false, 'Verify Code');
  }
}

function handleBack() {
  verifyForm.classList.add('hidden');
  loginForm.classList.remove('hidden');
  codeInput.value = '';
  pendingAuthId = null;
}

function handleLogout() {
  authToken = null;
  currentUser = null;
  localStorage.removeItem('tnnl_token');
  localStorage.removeItem('tnnl_user');

  // Show login screen
  mainScreen.style.display = 'none';
  loginScreen.style.display = 'block';

  // Reset forms
  emailInput.value = '';
  codeInput.value = '';
  verifyForm.classList.add('hidden');
  loginForm.classList.remove('hidden');
}

function showMainScreen() {
  loginScreen.style.display = 'none';
  mainScreen.style.display = 'block';

  if (currentUser) {
    userEmailEl.textContent = currentUser.email;
  }

  // Initialize app
  init();
}

async function checkAuth() {
  const token = localStorage.getItem('tnnl_token');
  const userJson = localStorage.getItem('tnnl_user');

  if (token && userJson) {
    authToken = token;
    currentUser = JSON.parse(userJson);
    showMainScreen();
  } else {
    // Not logged in - show the settings window so user can log in
    try {
      await invoke('show_and_activate_window');
    } catch (error) {
      console.error('Failed to show window:', error);
    }
  }
}

// Auth Event Listeners
loginForm.addEventListener('submit', handleLogin);
verifyForm.addEventListener('submit', handleVerify);
backBtn.addEventListener('click', handleBack);
logoutBtn.addEventListener('click', handleLogout);

// ============================================================================
// MAIN APP FUNCTIONS
// ============================================================================

function updateStatus(message: string) {
  statusEl.textContent = message;
  console.log('[tnnl]', message);
}

async function updateStats() {
  try {
    const status = await invoke<CaptureStatus>('get_capture_status');

    if (status.is_capturing && statsEl) {
      statsEl.innerHTML = `
        <strong>Capturing:</strong> ${status.frame_count} frames<br>
        <strong>Duration:</strong> ${status.elapsed_seconds.toFixed(1)}s<br>
        <strong>Avg FPS:</strong> ${status.average_fps}
      `;
    } else if (statsEl) {
      statsEl.innerHTML = '<em>Not capturing</em>';
    }
  } catch (error) {
    console.error('Failed to get status:', error);
  }
}

async function checkPermissions() {
  try {
    const hasPermission = await invoke<boolean>('check_permissions');
    if (!hasPermission) {
      updateStatus('⚠️  Screen recording permission required');
    }
  } catch (error) {
    console.error('Permission check failed:', error);
  }
}

async function startCapture() {
  try {
    updateStatus('Starting screen capture...');
    const result = await invoke<string>('start_screen_capture');

    startBtn.disabled = true;
    stopBtn.disabled = false;
    updateStatus('✓ Screen capture active');

    if (!statusInterval) {
      statusInterval = window.setInterval(updateStats, 1000);
    }

    console.log(result);
  } catch (error) {
    updateStatus(`❌ Error: ${error}`);
    console.error('Failed to start capture:', error);
  }
}

async function stopCapture() {
  try {
    updateStatus('Stopping screen capture...');
    const result = await invoke<string>('stop_screen_capture');

    startBtn.disabled = false;
    stopBtn.disabled = true;
    updateStatus('✓ Screen capture stopped');

    if (statusInterval) {
      clearInterval(statusInterval);
      statusInterval = null;
    }

    if (statsEl) {
      statsEl.innerHTML = '<em>Not capturing</em>';
    }

    console.log(result);
  } catch (error) {
    updateStatus(`❌ Error: ${error}`);
    console.error('Failed to stop capture:', error);
  }
}

async function connectToTunnel() {
  if (!authToken) {
    tunnelInfoEl.innerHTML = '<em style="color: #dc2626;">Not authenticated</em>';
    return;
  }

  try {
    connectTunnelBtn.disabled = true;
    connectTunnelBtn.textContent = 'Connecting...';
    tunnelInfoEl.innerHTML = '<em>Connecting to coordination server...</em>';

    const password = tunnelPasswordInput.value.trim();
    const passwordParam = password ? password : null;

    await invoke<string>('connect_to_coordination_server', {
      accessToken: authToken,
      password: passwordParam
    });

    console.log('[Tunnel] Connection initiated');

    // Poll for tunnel info
    let attempts = 0;
    const maxAttempts = 30;
    const pollInterval = setInterval(async () => {
      attempts++;
      try {
        const tunnelInfo = await invoke<TunnelInfo | null>('get_tunnel_info');

        if (tunnelInfo) {
          clearInterval(pollInterval);
          connectTunnelBtn.disabled = false;
          connectTunnelBtn.textContent = 'Reconnect';
          tunnelPasswordInput.disabled = true;

          tunnelInfoEl.innerHTML = `
            <strong>✓ Connected</strong><br>
            <strong>URL:</strong> <a href="${tunnelInfo.url}" target="_blank" style="color: #ffffff; text-decoration: underline;">${tunnelInfo.url}</a><br>
            <strong>Port:</strong> ${tunnelInfo.port}<br>
            ${tunnelInfo.password ? '<strong>Password:</strong> Protected (username: tnnl)<br>' : '<em>No password required</em>'}
          `;
        } else if (attempts >= maxAttempts) {
          clearInterval(pollInterval);
          connectTunnelBtn.disabled = false;
          connectTunnelBtn.textContent = 'Connect to tnnl.to';
          tunnelInfoEl.innerHTML = '<em style="color: #dc2626;">Connection timeout. Please try again.</em>';
        }
      } catch (error) {
        console.error('[Tunnel] Status check failed:', error);
      }
    }, 1000);

  } catch (error: any) {
    console.error('[Tunnel] Connection failed:', error);
    connectTunnelBtn.disabled = false;
    connectTunnelBtn.textContent = 'Connect to tnnl.to';
    tunnelInfoEl.innerHTML = `<em style="color: #dc2626;">Error: ${error}</em>`;
  }
}

async function updateTunnelInfo() {
  try {
    const tunnelInfo = await invoke<TunnelInfo | null>('get_tunnel_info');

    if (tunnelInfo && tunnelInfoEl) {
      connectTunnelBtn.textContent = 'Reconnect';
      tunnelPasswordInput.disabled = true;
      tunnelInfoEl.innerHTML = `
        <strong>✓ Connected</strong><br>
        <strong>URL:</strong> <a href="${tunnelInfo.url}" target="_blank" style="color: #ffffff; text-decoration: underline;">${tunnelInfo.url}</a><br>
        <strong>Port:</strong> ${tunnelInfo.port}<br>
        ${tunnelInfo.password ? '<strong>Password:</strong> Protected (username: tnnl)<br>' : '<em>No password required</em>'}
      `;
    }
  } catch (error) {
    // Not connected yet, ignore
  }
}

async function syncUIState() {
  try {
    const status = await invoke<CaptureStatus>('get_capture_status');
    if (status.is_capturing) {
      startBtn.disabled = true;
      stopBtn.disabled = false;
      updateStatus('✓ Screen capture active (auto-started)');

      if (!statusInterval) {
        statusInterval = window.setInterval(updateStats, 1000);
      }
      updateStats();
    } else {
      updateStatus('Ready to start');
    }
  } catch (error) {
    console.error('Failed to check capture status:', error);
  }

  // Check tunnel status
  try {
    await updateTunnelInfo();
  } catch (error) {
    console.error('Failed to check tunnel status:', error);
  }
}

// Main App Event Listeners
startBtn.addEventListener('click', startCapture);
stopBtn.addEventListener('click', stopCapture);
connectTunnelBtn.addEventListener('click', connectToTunnel);
togglePasswordBtn.addEventListener('click', () => {
  const isPassword = tunnelPasswordInput.type === 'password';
  tunnelPasswordInput.type = isPassword ? 'text' : 'password';
  togglePasswordBtn.textContent = isPassword ? 'Hide' : 'Show';
  togglePasswordBtn.title = isPassword ? 'Hide password' : 'Show password';
});

// Initialize Main App
async function init() {
  updateStatus('Initializing...');
  await checkPermissions();
  await syncUIState();

  // Poll for state changes every 2 seconds
  setInterval(syncUIState, 2000);
}

// Check auth on load
checkAuth();
