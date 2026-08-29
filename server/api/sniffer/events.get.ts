import { createEventStream } from 'h3';
import { snifferSession } from '../../../src/sniffer-session';

export default defineEventHandler(async (event) => {
  const eventStream = createEventStream(event);
  const unsubscribe = snifferSession.subscribe((payload) => {
    void eventStream.push(JSON.stringify(payload));
  });

  eventStream.onClosed(() => {
    unsubscribe();
  });

  return eventStream.send();
});
