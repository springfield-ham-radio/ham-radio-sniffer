import { snifferSession } from '../../../src/sniffer-session';

export default defineEventHandler(() => {
  return {
    status: snifferSession.getStatus(),
    packets: snifferSession.getPackets(),
    file: {
      path: snifferSession.getStatus().logFile,
      data: snifferSession.getLogData(),
    },
  };
});
