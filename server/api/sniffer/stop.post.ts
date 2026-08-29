import { snifferSession } from '../../../src/sniffer-session';

export default defineEventHandler(() => {
  return snifferSession.stop();
});
