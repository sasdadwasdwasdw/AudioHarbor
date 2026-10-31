import { describe, expect, it } from 'vitest';
import { validUrl, sourceName, duration, bytes, active, retryable } from './utils';
describe('UI formatting and URL feedback', () => {
  it('recognizes complete URLs and rejects shell text', () => { expect(validUrl('https://youtu.be/a')).toBe(true); expect(validUrl('file:///C:/secret')).toBe(false); expect(validUrl('--exec calc')).toBe(false); expect(validUrl('https://user:pass@example.com')).toBe(false); });
  it('labels sources without treating a lookalike as YouTube', () => { expect(sourceName('https://youtube.com.example.com/a')).toBe('youtube.com.example.com'); expect(sourceName('https://youtu.be/a')).toBe('YouTube'); });
  it('formats unknown and long progress data', () => { expect(duration(null)).toBe('—'); expect(duration(3670)).toBe('1:01:10'); expect(bytes(1024)).toBe('1.0 KB'); expect(bytes(null)).toBe('—'); });
  it('offers retry only for terminal failures', () => { expect(active('converting')).toBe(true); expect(retryable('interrupted')).toBe(true); expect(retryable('completed')).toBe(false); });
});
