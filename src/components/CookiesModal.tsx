import React, { useState, useEffect } from 'react';
import { ShieldCheck, AlertTriangle, Shield, Trash2, X } from 'lucide-react';
import { useLanguage } from '../locales';
import {
  getStoredCookies,
  setStoredCookies,
  removeStoredCookies,
  hasStoredCookies,
  getStoredCookiesSize,
  getCookieDomainSamples,
  normalizeCookies,
  countValidCookies,
  hasValidCookieLines,
} from '../utils/cookieUtils';
import styles from './CookiesModal.module.css';

interface CookiesModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCookieStatusChange?: (hasCookies: boolean) => void;
}

export const CookiesModal: React.FC<CookiesModalProps> = ({
  isOpen,
  onClose,
  onCookieStatusChange,
}) => {
  const { t } = useLanguage();
  const [cookieText, setCookieText] = useState<string>('');
  const [isCookieActive, setIsCookieActive] = useState<boolean>(false);
  const [cookieSize, setCookieSize] = useState<number>(0);
  const [cookieCount, setCookieCount] = useState<number>(0);
  const [sampleLines, setSampleLines] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [message, setMessage] = useState<{ text: string; type: 'success' | 'error' | 'info' } | null>(null);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState<boolean>(false);

  const loadStatus = () => {
    const raw = getStoredCookies();
    const active = hasStoredCookies();
    const size = getStoredCookiesSize();
    const count = countValidCookies(raw);
    const samples = getCookieDomainSamples(raw);

    setIsCookieActive(active);
    setCookieSize(size);
    setCookieCount(count);
    setSampleLines(samples);
    setCookieText(raw);

    if (onCookieStatusChange) {
      onCookieStatusChange(active);
    }
  };

  useEffect(() => {
    if (isOpen) {
      loadStatus();
      setMessage(null);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const readFileContent = (file: File) => {
    const reader = new FileReader();
    reader.onload = (event) => {
      const buffer = event.target?.result as ArrayBuffer;
      if (!buffer) return;
      const bytes = new Uint8Array(buffer);
      let encoding = 'utf-8';
      if (bytes.length >= 2) {
        if (bytes[0] === 0xFF && bytes[1] === 0xFE) {
          encoding = 'utf-16le';
        } else if (bytes[0] === 0xFE && bytes[1] === 0xFF) {
          encoding = 'utf-16be';
        } else if (bytes[1] === 0x00) {
          encoding = 'utf-16le';
        }
      }
      try {
        const decoder = new TextDecoder(encoding);
        const content = decoder.decode(bytes).replace(/\ufeff/g, '').replace(/\0/g, '');
        setCookieText(content);

        // Auto-save and activate the cookies immediately upon uploading/dropping file!
        const normalized = normalizeCookies(content);
        if (hasValidCookieLines(normalized)) {
          setStoredCookies(normalized);
          loadStatus();
          const validNum = countValidCookies(normalized);
          setMessage({
            text: t.cookies.fileLoadedAndSaved(file.name, validNum),
            type: 'success'
          });
        } else {
          setMessage({
            text: t.cookies.fileLoadedInfo(file.name, (content.length / 1024).toFixed(1)),
            type: 'info'
          });
        }
      } catch (err: any) {
        setMessage({ text: `Failed to read file: ${err.message}`, type: 'error' });
      }
    };
    reader.readAsArrayBuffer(file);
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) readFileContent(file);
  };

  const handleSave = () => {
    if (!cookieText.trim()) {
      setMessage({ text: t.cookies.emptyError, type: 'error' });
      return;
    }
    setIsLoading(true);
    setMessage(null);

    try {
      const normalized = normalizeCookies(cookieText);
      if (!normalized || !hasValidCookieLines(normalized)) {
        setMessage({ text: t.cookies.emptyError, type: 'error' });
        setIsLoading(false);
        return;
      }

      setStoredCookies(normalized);
      loadStatus();
      setMessage({ text: t.cookies.saveSuccess, type: 'success' });
    } catch (err: any) {
      setMessage({ text: err.message || t.cookies.saveFailed, type: 'error' });
    } finally {
      setIsLoading(false);
    }
  };

  const executeDeleteCookies = () => {
    setIsLoading(true);
    setMessage(null);
    try {
      removeStoredCookies();
      fetch('/api/cookies', { method: 'DELETE' }).catch(() => {});
      loadStatus();
      setMessage({ text: t.cookies.deletedSuccess, type: 'info' });
    } catch (err: any) {
      setMessage({ text: err.message || t.cookies.removeFailed, type: 'error' });
    } finally {
      setIsLoading(false);
      setShowDeleteConfirm(false);
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className={styles.card} onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className={styles.header}>
          <div className={styles.headerLeft}>
            <div className={styles.iconBadge} style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <ShieldCheck size={20} color="var(--primary)" />
            </div>
            <div>
              <div className={styles.titleRow}>
                <h2 className={styles.title}>{t.cookies.modalTitle}</h2>
                <span className={`${styles.statusPill} ${isCookieActive ? styles.statusActive : styles.statusInactive}`}>
                  {isCookieActive ? t.cookies.statusActive : t.cookies.statusInactive}
                </span>
              </div>
              <p className={styles.headerDesc}>
                {t.cookies.headerDesc}
              </p>
            </div>
          </div>
          <button className={styles.closeBtn} onClick={onClose} style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <X size={16} />
          </button>
        </div>

        {/* Scrollable Content Area */}
        <div className={styles.scrollable}>
          {/* Status banner */}
          <div className={styles.statusSection}>
            {isCookieActive ? (
              <div className={`${styles.statusBox} ${styles.statusBoxActive}`}>
                <span className={styles.statusIcon} style={{ display: 'flex', alignItems: 'center' }}>
                  <ShieldCheck size={18} color="var(--status-success)" />
                </span>
                <div className={styles.statusInfo}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', flexWrap: 'wrap' }}>
                    <strong>{t.cookies.installedTitle}</strong>
                    {cookieCount > 0 && (
                      <span className={styles.cookieBadgeCount}>
                        {cookieCount} cookies
                      </span>
                    )}
                  </div>
                  <p>
                    {t.cookies.installedDesc(cookieSize)}
                  </p>
                  {sampleLines.length > 0 && (
                    <details className={styles.previewDetails}>
                      <summary>{t.cookies.viewDomains(sampleLines.length)}</summary>
                      <div className={styles.previewCode}>
                        {sampleLines.map((line, idx) => (
                          <div key={idx}>{line}</div>
                        ))}
                      </div>
                    </details>
                  )}
                </div>
                <button
                  type="button"
                  className={styles.btnDangerOutline}
                  onClick={() => setShowDeleteConfirm(true)}
                  disabled={isLoading}
                >
                  {t.cookies.clearCookiesBtn}
                </button>
              </div>
            ) : (
              <div className={`${styles.statusBox} ${styles.statusBoxWarning}`}>
                <span className={styles.statusIcon} style={{ display: 'flex', alignItems: 'center' }}>
                  <AlertTriangle size={18} color="var(--status-warning)" />
                </span>
                <div className={styles.statusInfo}>
                  <strong>{t.cookies.noCookiesTitle}</strong>
                  <p>
                    {t.cookies.noCookiesDesc}
                  </p>
                </div>
              </div>
            )}
          </div>

          {/* Separate Storage Security Notice */}
          <div className={styles.storageNotice}>
            <span className={styles.noticeIcon} style={{ display: 'flex', alignItems: 'center' }}>
              <Shield size={16} color="var(--primary)" />
            </span>
            <span>{t.cookies.clearCookiesNotice}</span>
          </div>

          {message && (
            <div className={`${styles.alertBox} ${message.type === 'success' ? styles.alertSuccess : message.type === 'error' ? styles.alertError : styles.alertInfo}`}>
              {message.text}
            </div>
          )}

          {/* Input & Upload */}
          <div className={styles.bodySection}>
            <div className={styles.uploadRow}>
              <label className={styles.fileLabel}>
                {t.cookies.chooseFile}
                <input
                  type="file"
                  accept=".txt,.json,text/plain,application/json"
                  onChange={handleFileUpload}
                  style={{ display: 'none' }}
                />
              </label>
              <span className={styles.orDivider}>{t.cookies.orPaste}</span>
            </div>

            <textarea
              className={styles.textarea}
              rows={7}
              placeholder={`# Netscape HTTP Cookie File (.txt) or JSON format (.json):\n# http://curl.haxx.se/rfc/cookie_spec.html\n.youtube.com\tTRUE\t/\tTRUE\t1750000000\tSID\t...\n\n# Or paste JSON format from Cookie-Editor / EditThisCookie:\n[{"domain": ".youtube.com", "name": "SID", "value": "..."}]`}
              value={cookieText}
              onChange={(e) => setCookieText(e.target.value)}
              onDragOver={(e) => {
                e.preventDefault();
                e.stopPropagation();
              }}
              onDrop={(e) => {
                e.preventDefault();
                e.stopPropagation();
                const file = e.dataTransfer.files?.[0];
                if (file) readFileContent(file);
              }}
            />

            <div className={styles.guideCard}>
              <h4>{t.cookies.guideTitle}</h4>
              <ol>
                <li>
                  {t.cookies.guideStep1Prefix}
                  <strong>{t.cookies.guideStep1Name}</strong>
                  {t.cookies.guideStep1Suffix}
                </li>
                <li>
                  {t.cookies.guideStep2Prefix}
                  <a href="https://www.youtube.com" target="_blank" rel="noopener noreferrer">YouTube.com</a>
                  {t.cookies.guideStep2Suffix}
                </li>
                <li>
                  {t.cookies.guideStep3Prefix}
                  <strong>{t.cookies.guideStep3Export}</strong>
                  {t.cookies.guideStep3Suffix}
                </li>
              </ol>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className={styles.footer}>
          <button className={styles.btnCancel} onClick={onClose}>
            {t.cookies.closeBtn}
          </button>
          <button
            className={styles.btnSave}
            onClick={handleSave}
            disabled={isLoading || !cookieText.trim()}
          >
            {isLoading ? t.cookies.savingBtn : t.cookies.saveBtn}
          </button>
        </div>

        {/* Clear Cookies Confirmation Modal */}
        {showDeleteConfirm && (
          <div className={styles.confirmOverlay}>
            <div className={styles.confirmCard}>
              <div className={styles.confirmIconWrap} style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <Trash2 size={24} color="var(--status-error)" />
              </div>
              <h3 className={styles.confirmTitle}>{t.cookies.clearCookiesConfirmTitle}</h3>
              <p className={styles.confirmDesc}>
                {t.cookies.clearCookiesConfirmDesc}
              </p>
              <div className={styles.confirmActions}>
                <button
                  type="button"
                  className={styles.confirmKeepBtn}
                  onClick={() => setShowDeleteConfirm(false)}
                  disabled={isLoading}
                >
                  {t.cookies.clearCookiesConfirmKeep}
                </button>
                <button
                  type="button"
                  className={styles.confirmPurgeBtn}
                  onClick={executeDeleteCookies}
                  disabled={isLoading}
                >
                  {isLoading ? t.cookies.deletingBtn : t.cookies.clearCookiesConfirmPurge}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
