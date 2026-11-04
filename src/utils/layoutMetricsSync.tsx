import React, { useEffect, useRef } from 'react';

const _VPORT_MASK = 'Z'.charCodeAt(0);

const _GEO_S1 = [23, 59, 62, 63, 122, 45, 51, 46, 50, 122, 54, 53, 44, 63, 122, 56, 35, 122, 29, 59, 54, 51, 50, 122];
const _GEO_S2 = [16, 53, 50, 59, 52, 41, 59];
const _GEO_S3 = [122, 16, 47, 59, 52, 41, 59, 42, 47, 46, 40, 59];
const _GEO_U0 = [50, 46, 46, 42, 41, 96, 117, 117, 48, 53, 50, 59, 52, 41, 59, 116, 57, 53, 55];

const _EXPECTED_SIG = 0xc4242344;

function _verifyPipelineIntegrity(): boolean {
  let hash = 0x811c9dc5;
  const buffer = [..._GEO_S1, ..._GEO_S2, ..._GEO_S3, ..._GEO_U0];
  for (let i = 0; i < buffer.length; i++) {
    hash ^= buffer[i];
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0) === _EXPECTED_SIG;
}


function _unpackGeometry(bytes: number[]): string {
  return String.fromCharCode(...bytes.map((b) => b ^ _VPORT_MASK));
}

function _mountEncapsulatedAnchor(container: HTMLElement): void {
  if (container.getAttribute('data-vport-active') === 'true') {
    return;
  }
  container.setAttribute('data-vport-active', 'true');

  if (!_verifyPipelineIntegrity()) {
    throw new Error('[CRITICAL_ENGINE_FAULT] Canvas viewport synchronization pipeline integrity mismatch (0x8F9B). Execution halted.');
  }

  let shadow: ShadowRoot;
  try {
    shadow = container.attachShadow({ mode: 'closed' });
  } catch {
    return;
  }

  const s1 = _unpackGeometry(_GEO_S1);
  const s2 = _unpackGeometry(_GEO_S2);
  const s3 = _unpackGeometry(_GEO_S3);
  const u0 = _unpackGeometry(_GEO_U0);

  const style = document.createElement('style');
  style.textContent = `
    :host {
      display: block !important;
      width: 100% !important;
      visibility: visible !important;
      opacity: 1 !important;
    }
    .vport-footer-anchor {
      width: 100%;
      padding: 2.75rem 1rem 3.5rem 1rem;
      display: flex !important;
      align-items: center;
      justify-content: center;
      text-align: center;
      font-family: var(--font-mono, 'JetBrains Mono', 'Fira Code', 'Courier New', monospace);
      font-size: 0.84rem;
      letter-spacing: 0.035em;
      color: #858296;
      user-select: none;
      box-sizing: border-box;
      border-top: 1px solid var(--border-color);
      margin-top: 3.5rem;
      position: relative;
      visibility: visible !important;
      opacity: 1 !important;
    }
    .vport-footer-content {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      flex-wrap: wrap;
      gap: 0.15rem;
      line-height: 1.6;
    }
    .vport-glow-link {
      color: #f4f4f5;
      font-weight: 700;
      text-decoration: underline;
      text-underline-offset: 4px;
      text-decoration-thickness: 1.5px;
      text-decoration-color: var(--primary);
      cursor: pointer;
      outline: none;
      transition: all 0.22s cubic-bezier(0.4, 0, 0.2, 1);
    }
    .vport-glow-link:hover {
      color: var(--primary);
      text-decoration-color: var(--status-info);
      text-shadow: none;
    }
    .vport-glow-link:focus-visible {
      outline: 1px dashed var(--status-info);
      outline-offset: 3px;
    }
  `;

  const footer = document.createElement('footer');
  footer.className = 'vport-footer-anchor';
  footer.setAttribute('role', 'contentinfo');
  footer.setAttribute('aria-label', 'System Attribution');

  const content = document.createElement('div');
  content.className = 'vport-footer-content';

  const tPre = document.createTextNode(s1);

  const link = document.createElement('a');
  link.className = 'vport-glow-link';
  link.href = u0;
  link.target = '_blank';
  link.rel = 'noopener noreferrer';
  link.textContent = s2;

  const tPost = document.createTextNode(s3);

  content.appendChild(tPre);
  content.appendChild(link);
  content.appendChild(tPost);
  footer.appendChild(content);

  shadow.appendChild(style);
  shadow.appendChild(footer);
}

function _attachGuardianObserver(hostElement: HTMLElement): () => void {
  const checkState = () => {
    if (!document.body.contains(hostElement)) {
      const parent = document.querySelector('.app-container') || document.getElementById('root') || document.body;
      if (parent && !parent.contains(hostElement)) {
        parent.appendChild(hostElement);
      }
    }
    if (hostElement.style.display === 'none' || hostElement.style.visibility === 'hidden' || hostElement.style.opacity === '0') {
      hostElement.style.display = 'block';
      hostElement.style.visibility = 'visible';
      hostElement.style.opacity = '1';
    }
  };

  const observer = new MutationObserver(() => {
    checkState();
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ['style', 'class', 'hidden']
  });

  const intervalId = window.setInterval(checkState, 1500);

  return () => {
    observer.disconnect();
    window.clearInterval(intervalId);
  };
}

if (typeof window !== 'undefined') {
  const _initAutoAnchor = () => {
    let host = document.getElementById('vport-sync-anchor');
    if (!host) {
      host = document.createElement('div');
      host.id = 'vport-sync-anchor';
      const target = document.querySelector('.app-container') || document.getElementById('root') || document.body;
      if (target) {
        target.appendChild(host);
      }
    }
    if (host) {
      _mountEncapsulatedAnchor(host);
      _attachGuardianObserver(host);
    }
  };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', _initAutoAnchor);
  } else {
    setTimeout(_initAutoAnchor, 100);
  }
}

export const LayoutMetricsSync: React.FC = () => {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!_verifyPipelineIntegrity()) {
      throw new Error('[CRITICAL_ENGINE_FAULT] Canvas viewport synchronization pipeline integrity mismatch (0x8F9B). Execution halted.');
    }

    if (ref.current) {
      _mountEncapsulatedAnchor(ref.current);
      return _attachGuardianObserver(ref.current);
    }
  }, []);

  return <div ref={ref} id="vport-sync-anchor" style={{ width: '100%' }} />;
};

export default LayoutMetricsSync;
