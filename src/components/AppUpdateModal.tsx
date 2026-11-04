import React, { useState, useEffect, useRef } from 'react';
import { RefreshCw, CheckCircle2, AlertTriangle, RotateCcw, X } from 'lucide-react';
import { useLanguage } from '../locales';
import styles from './AppUpdateModal.module.css';

export interface VersionInfo {
  current_commit: string;
  current_commit_full?: string;
  commit_message?: string;
  commit_date?: string;
  branch?: string;
  remote_url?: string;
  update_available?: boolean;
  behind_count?: number;
  remote_commit?: string;
  changelog?: Array<{ hash: string; message: string; date: string }>;
  error?: string;
  is_server?: boolean;
  allow_update?: boolean;
}

interface AppUpdateModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AppUpdateModal: React.FC<AppUpdateModalProps> = ({ isOpen, onClose }) => {
  const { t } = useLanguage();
  const [info, setInfo] = useState<VersionInfo | null>(null);
  const [isChecking, setIsChecking] = useState<boolean>(false);
  const [isUpdating, setIsUpdating] = useState<boolean>(false);
  const [isRestarting, setIsRestarting] = useState<boolean>(false);
  const [reconnectAttempt, setReconnectAttempt] = useState<number>(0);
  const [statusText, setStatusText] = useState<string>('');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const pollTimerRef = useRef<number | null>(null);

  // Clear polling timer on unmount
  useEffect(() => {
    return () => {
      if (pollTimerRef.current) {
        clearInterval(pollTimerRef.current);
      }
    };
  }, []);

  const fetchVersion = async () => {
    try {
      const res = await fetch('/api/system/version');
      if (res.ok) {
        const data = await res.json();
        setInfo((prev) => ({ ...prev, ...data }));
      }
    } catch {
      // Ignored
    }
  };

  const handleCheckUpdate = async () => {
    setIsChecking(true);
    setErrorMessage(null);
    try {
      const res = await fetch('/api/system/check-update');
      if (res.ok) {
        const data = await res.json();
        setInfo(data);
        if (data.error) {
          setErrorMessage(data.error);
        }
      } else {
        setErrorMessage(t.updateModal.errorTitle);
      }
    } catch (err: any) {
      setErrorMessage(err.message || 'Network error while checking updates');
    } finally {
      setIsChecking(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      fetchVersion();
      handleCheckUpdate();
    } else {
      setErrorMessage(null);
      setIsUpdating(false);
      setIsRestarting(false);
      setReconnectAttempt(0);
      if (pollTimerRef.current) {
        clearInterval(pollTimerRef.current);
        pollTimerRef.current = null;
      }
    }
  }, [isOpen]);

  const startPollingReconnect = () => {
    setIsRestarting(true);
    setStatusText(t.updateModal.reconnecting);
    let attempts = 0;

    // Wait 3.5 seconds before starting polling so the old processes have time to shut down
    setTimeout(() => {
      pollTimerRef.current = setInterval(async () => {
        attempts += 1;
        setReconnectAttempt(attempts);
        try {
          const res = await fetch('/api/health?refresh=true', { cache: 'no-store' });
          if (res.ok) {
            if (pollTimerRef.current) {
              clearInterval(pollTimerRef.current);
              pollTimerRef.current = null;
            }
            setStatusText(t.updateModal.reconnected);
            setTimeout(() => {
              window.location.reload();
            }, 800);
          }
        } catch {
          // Still waiting for restart
        }

        if (attempts >= 45) {
          if (pollTimerRef.current) {
            clearInterval(pollTimerRef.current);
            pollTimerRef.current = null;
          }
          setErrorMessage('Restart took longer than expected. Please manually refresh your browser window.');
          setIsRestarting(false);
        }
      }, 1500);
    }, 3500);
  };

  const handleUpdateAndRestart = async () => {
    setIsUpdating(true);
    setErrorMessage(null);
    setStatusText(t.updateModal.updatingApp);

    try {
      const res = await fetch('/api/system/update', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.detail || data.message || 'Update failed');
      }
      setIsUpdating(false);
      startPollingReconnect();
    } catch (err: any) {
      setIsUpdating(false);
      setErrorMessage(err.message || 'Error executing update');
    }
  };

  const handleDirectRestart = async () => {
    setErrorMessage(null);
    try {
      await fetch('/api/system/restart', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      });
      startPollingReconnect();
    } catch {
      startPollingReconnect();
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className={styles.overlay}
      onClick={(e) => {
        if (e.target === e.currentTarget && !isUpdating && !isRestarting) {
          onClose();
        }
      }}
    >
      <div className={styles.card}>
        {/* Modal Header */}
        <div className={styles.header}>
          <div className={styles.headerLeft}>
            <span className={styles.headerIcon} style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <RefreshCw size={18} color="var(--primary)" />
            </span>
            <div>
              <h3 className={styles.title}>{t.updateModal.title}</h3>
              <p className={styles.subtitle}>{t.updateModal.subtitle}</p>
            </div>
          </div>
          {!isUpdating && !isRestarting && (
            <button
              onClick={onClose}
              className={styles.closeBtn}
              title={t.updateModal.closeBtn}
              style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}
            >
              <X size={16} />
            </button>
          )}
        </div>

        {/* Modal Content */}
        <div className={styles.body}>
          {/* Active Restarting Overlay / Screen */}
          {isRestarting ? (
            <div className={styles.progressContainer}>
              <div className={styles.spinnerPrimary} />
              <div>
                <h4 className={styles.progressTitle}>
                  {t.updateModal.restartingTitle}
                </h4>
                <p className={styles.progressDesc}>
                  {t.updateModal.restartingDesc}
                </p>
              </div>
              <div className={styles.reconnectBadge}>
                <span>{statusText || t.updateModal.reconnecting}</span>
                {reconnectAttempt > 0 && <span>({reconnectAttempt})</span>}
              </div>
            </div>
          ) : isUpdating ? (
            /* Active Updating Progress */
            <div className={styles.progressContainer}>
              <div className={styles.spinnerSuccess} />
              <div>
                <h4 className={styles.progressTitle}>
                  {t.updateModal.updatingApp}
                </h4>
                <p className={styles.progressDesc}>
                  Pulling latest commits from GitHub and preparing restart...
                </p>
              </div>
            </div>
          ) : (
            <>
              {/* Version Information Card */}
              <div className={styles.versionCard}>
                <div className={styles.versionTop}>
                  <span className={styles.versionLabel}>
                    {t.updateModal.currentVersion}
                  </span>
                  <div style={{ display: 'flex', gap: '0.4rem', alignItems: 'center' }}>
                    <span className={styles.commitBadge}>
                      {info?.current_commit || '...'}
                    </span>
                    <span className={styles.branchBadge}>
                      {info?.branch || 'master'}
                    </span>
                  </div>
                </div>

                {info?.commit_message && (
                  <div className={styles.commitMessage}>
                    "{info.commit_message}"
                  </div>
                )}

                {info?.commit_date && (
                  <div className={styles.commitDate}>
                    {t.updateModal.lastUpdated}: {info.commit_date}
                  </div>
                )}
              </div>

              {/* Update Status Banner */}
              {isChecking ? (
                <div className={styles.checkingCard}>
                  <div className={styles.spinnerSm} />
                  <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                    {t.updateModal.checkingUpdate}
                  </span>
                </div>
              ) : info?.allow_update === false ? (
                /* Server Deployment Notice */
                <div className={styles.serverDeployCard}>
                  <span style={{ fontSize: '1.3rem', color: 'var(--status-info)' }}>[SEC]</span>
                  <div>
                    <h4 style={{ margin: 0, fontSize: '0.88rem', color: 'var(--status-info)', fontWeight: 600 }}>
                      Server / Production Deployment
                    </h4>
                    <p style={{ margin: '5px 0 0 0', fontSize: '0.78rem', color: 'var(--text-secondary)', lineHeight: 1.5 }}>
                      Self-update and server restarts via the Web UI are disabled on remote servers for security. Please deploy updates through your container orchestrator (e.g. Dokploy / Docker / Git push).
                    </p>
                  </div>
                </div>
              ) : info?.update_available ? (
                <div className={styles.availableCard}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <span style={{ fontSize: '1.2rem', fontFamily: 'var(--font-mono)', color: 'var(--status-success)' }}>&gt;</span>
                    <div>
                      <h4 style={{ margin: 0, fontSize: '0.95rem', color: 'var(--status-success)', fontWeight: 700 }}>
                        {t.updateModal.updateAvailableTitle}
                      </h4>
                      <p style={{ margin: '2px 0 0 0', fontSize: '0.78rem', color: 'var(--text-secondary)' }}>
                        {t.updateModal.updateAvailableDesc(info.behind_count || 1)}
                      </p>
                    </div>
                  </div>

                  {/* Changelog list */}
                  {info.changelog && info.changelog.length > 0 && (
                    <div style={{ marginTop: '0.25rem' }}>
                      <span style={{ fontSize: '0.78rem', color: 'var(--text-secondary)', fontWeight: 600, display: 'block', marginBottom: '0.35rem' }}>
                        {t.updateModal.newCommits}
                      </span>
                      <div className={styles.changelogBox}>
                        {info.changelog.map((c, i) => (
                          <div key={i} style={{ fontSize: '0.78rem', display: 'flex', gap: '0.5rem', alignItems: 'flex-start' }}>
                            <span className={styles.changelogHash}>
                              {c.hash}
                            </span>
                            <span style={{ color: 'var(--text-primary)', wordBreak: 'break-word' }}>
                              {c.message}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Primary Update & Restart Action */}
                  <button
                    onClick={handleUpdateAndRestart}
                    className={styles.btnPrimary}
                  >
                    <span>* {t.updateModal.updateRestartBtn}</span>
                  </button>
                </div>
              ) : (
                /* Up to date Banner */
                <div className={styles.uptodateCard}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.65rem' }}>
                    <CheckCircle2 size={20} color="var(--status-success)" style={{ flexShrink: 0 }} />
                    <div>
                      <h4 style={{ margin: 0, fontSize: '0.9rem', color: 'var(--status-success)', fontWeight: 600 }}>
                        {t.updateModal.upToDateTitle}
                      </h4>
                      <p style={{ margin: '2px 0 0 0', fontSize: '0.78rem', color: 'var(--text-secondary)' }}>
                        {t.updateModal.upToDateDesc}
                      </p>
                    </div>
                  </div>
                  <button
                    onClick={handleCheckUpdate}
                    disabled={isChecking}
                    className={styles.checkBtn}
                  >
                    {isChecking ? t.updateModal.checkingAgain : t.updateModal.checkUpdateBtn}
                  </button>
                </div>
              )}

              {/* Error Alert */}
              {errorMessage && (
                <div className={styles.errorBanner} style={{ display: 'flex', alignItems: 'center', gap: '0.35rem' }}>
                  <AlertTriangle size={15} color="var(--status-error)" style={{ flexShrink: 0 }} />
                  <span>{errorMessage}</span>
                </div>
              )}
            </>
          )}
        </div>

        {/* Modal Footer */}
        {!isUpdating && !isRestarting && (
          <div className={styles.footer}>
            {info?.allow_update !== false && (
              <button
                onClick={handleDirectRestart}
                className={styles.restartBtn}
                title="Restart backend and frontend dev server"
                style={{ display: 'inline-flex', alignItems: 'center', gap: '0.35rem' }}
              >
                <RotateCcw size={13} />
                <span>{t.updateModal.restartBtn}</span>
              </button>
            )}

            <button
              onClick={onClose}
              className={styles.closeFooterBtn}
            >
              {t.updateModal.closeBtn}
            </button>
          </div>
        )}
      </div>
    </div>
  );
};
