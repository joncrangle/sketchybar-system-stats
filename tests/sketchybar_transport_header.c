#include "../include/sketchybar.h"

/* This second translation unit catches definitions accidentally left public. */
int sketchybar_header_translation_unit(void) {
  enum sketchybar_send_status failed = SKETCHYBAR_SEND_FAILED;
  enum sketchybar_send_status acknowledged = SKETCHYBAR_SEND_ACKNOWLEDGED;
  enum sketchybar_send_status no_ack = SKETCHYBAR_SENT_NO_ACK;
  return (int)failed + (int)acknowledged + (int)no_ack;
}
