import { parseStartSnifferRequest, StartRequestError } from '../../../src/parse-start-request';
import { SnifferConflictError, snifferSession } from '../../../src/sniffer-session';

export default defineEventHandler(async (event) => {
  const body = await readBody(event);

  try {
    const request = parseStartSnifferRequest(body);
    return snifferSession.start(request);
  } catch (error) {
    if (error instanceof StartRequestError) {
      throw createError({
        statusCode: 400,
        statusMessage: error.message,
      });
    }

    if (error instanceof SnifferConflictError) {
      throw createError({
        statusCode: 409,
        statusMessage: error.message,
      });
    }

    throw error;
  }
});
