import { snifferHealthPayload } from '../../shared/types/sniffer';

export default defineEventHandler(() => {
  const config = useRuntimeConfig();
  return snifferHealthPayload(String(config.public.snifferVersion ?? ''));
});
