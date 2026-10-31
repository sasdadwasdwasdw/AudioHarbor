import type { Settings } from './types';
const en = { downloader: 'Downloader', queue: 'Queue', history: 'History', settings: 'Settings', about: 'About & Updates', download: 'Add to queue', inspect: 'Find audio', paste: 'Paste link', source: 'Source link', folder: 'Save to', format: 'Audio format', quality: 'MP3 bitrate ceiling', ready: 'Ready when you are', local: 'Runs on your device', appearance: 'Appearance', language: 'Language', welcome: 'Good things, on repeat.', subtitle: 'Bring the audio you love a little closer. Paste a link to get started.' };
type Messages = typeof en;
const th: Messages = { downloader: 'ดาวน์โหลด', queue: 'คิว', history: 'ประวัติ', settings: 'ตั้งค่า', about: 'เกี่ยวกับและอัปเดต', download: 'เพิ่มลงคิว', inspect: 'ค้นหาเสียง', paste: 'วางลิงก์', source: 'ลิงก์ต้นทาง', folder: 'บันทึกไปที่', format: 'รูปแบบเสียง', quality: 'อัตราบิตสูงสุด MP3', ready: 'พร้อมเมื่อคุณพร้อม', local: 'ทำงานบนอุปกรณ์ของคุณ', appearance: 'ธีม', language: 'ภาษา', welcome: 'เสียงที่ชอบ ฟังซ้ำได้เสมอ', subtitle: 'นำเสียงที่คุณรักมาใกล้อีกนิด วางลิงก์เพื่อเริ่มต้น' };
// Typed catalogs and Intl formatting are ready for complete Thai localization.
// Secondary explanatory strings remain English in this initial release.
export const messages = (language: Settings['language'] = 'en'): Messages => language === 'th' ? th : en;
