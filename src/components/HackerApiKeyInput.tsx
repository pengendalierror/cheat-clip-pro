import React, { useState } from 'react';
import { Eye, EyeOff, X } from 'lucide-react';
import styles from './HackerApiKeyInput.module.css';

interface HackerApiKeyInputProps {
  id?: string;
  value: string;
  onChange: (val: string) => void;
  placeholder?: string;
  disabled?: boolean;
  error?: boolean;
  showToggle?: boolean;
  revealLabel?: string;
  maskLabel?: string;
}

export const HackerApiKeyInput: React.FC<HackerApiKeyInputProps> = ({
  id,
  value,
  onChange,
  placeholder = 'ENTER API KEY...',
  disabled = false,
  error = false,
  showToggle = true,
  revealLabel = 'Reveal key',
  maskLabel = 'Hide key',
}) => {
  const [isRevealed, setIsRevealed] = useState(false);

  return (
    <div
      className={`${styles.container} ${error ? styles.hasError : ''}`}
    >
      {/* Terminal prefix badge */}
      <div className={styles.prefix}>
        Key
      </div>

      {/* Input container with ASCII # masking */}
      <div className={styles.inputWrap}>
        <input
          id={id}
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
          placeholder={placeholder}
          autoComplete="off"
          spellCheck={false}
          className={`${styles.input} ${isRevealed ? styles.isRevealed : ''}`}
        />

        {/* Masked ASCII overlay layer when not revealed and has value */}
        {!isRevealed && value && (
          <div
            aria-hidden="true"
            className={styles.mask}
          >
            {'•'.repeat(Math.max(16, Math.min(value.length, 38)))}
          </div>
        )}
      </div>

      {/* Action buttons: Clear & Reveal toggle */}
      <div className={styles.actions}>
        {value && !disabled && (
          <button
            type="button"
            onClick={() => onChange('')}
            title="Clear key"
            className={styles.clearBtn}
          >
            <X size={14} />
          </button>
        )}

        {showToggle && (
          <button
            type="button"
            onClick={() => setIsRevealed(!isRevealed)}
            title={isRevealed ? maskLabel : revealLabel}
            className={`${styles.toggleBtn} ${isRevealed ? styles.isActive : ''}`}
          >
            {isRevealed ? <EyeOff size={14} /> : <Eye size={14} />}
            <span style={{ fontSize: '0.72rem' }}>{isRevealed ? 'Hide' : 'Show'}</span>
          </button>
        )}
      </div>
    </div>
  );
};
export default HackerApiKeyInput;
