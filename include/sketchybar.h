#pragma once

#include <stdbool.h>

enum sketchybar_send_status {
  SKETCHYBAR_SEND_FAILED = 0,
  SKETCHYBAR_SEND_ACKNOWLEDGED = 1,
  SKETCHYBAR_SENT_NO_ACK = 2,
};

/*
 * Sends a tokenized command to SketchyBar. The status describes queue
 * delivery: an acknowledgement does not prove that SketchyBar handled the
 * event. SENT_NO_ACK means the send succeeded but receiving any reply failed.
 *
 * If response is non-NULL, *response is set to NULL on entry. Callers must free
 * any allocation previously stored in *response before reusing it. When
 * available, the reply is newly allocated and must be freed with
 * free_sketchybar_response(). The result may remain NULL after an acknowledged
 * send if there was no reply payload or copying the payload failed. Passing
 * NULL still waits for an acknowledgement, but does not copy the reply.
 */
enum sketchybar_send_status
sketchybar_send(const char *message, const char *bar_name, char **response);

/* Legacy response-returning interface. Free its result with
 * free_sketchybar_response(). */
char *sketchybar(const char *message, const char *bar_name);

void free_sketchybar_response(char *response);
bool refresh_sketchybar_port(const char *bar_name);
void cleanup_sketchybar(void);
