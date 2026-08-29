import { listSerialPorts } from '../../src/list-ports';

export default defineEventHandler(async () => {
  return {
    ports: await listSerialPorts(),
  };
});
