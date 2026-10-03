#include <assert.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#define bootstrap_look_up probe_lookup
#include "../include/sketchybar.h"
#undef bootstrap_look_up

static mach_port_t private_port;
static int received_messages;

kern_return_t probe_lookup(mach_port_t bootstrap_port, const char *name,
                           mach_port_t *port) {
  (void)bootstrap_port;
  (void)name;
  *port = private_port;
  return mach_port_mod_refs(mach_task_self(), private_port,
                            MACH_PORT_RIGHT_SEND, 1);
}

static void assert_tokens(const char *input, const char *expected,
                          size_t expected_len) {
  char output[512] = {0};
  size_t output_len = tokenize_sketchybar_message(input, output);
  assert(output_len == expected_len);
  assert(memcmp(output, expected, expected_len) == 0);
  assert(output[output_len - 1] == '\0');
  assert(output[output_len - 2] == '\0');
}

static void test_tokenizer(void) {
  const char empty_expected[] = "\0";
  assert_tokens("", empty_expected, sizeof(empty_expected));
  assert_tokens(" \t", empty_expected, sizeof(empty_expected));

  const char plain_expected[] = "one\0";
  assert_tokens("one", plain_expected, sizeof(plain_expected));
  assert_tokens("one  \t", plain_expected, sizeof(plain_expected));

  const char payload_expected[] =
      "--trigger\0system_stats\0HOST_NAME=Alice's Mac\0CPU_USAGE=10%\0";
  assert_tokens(
      "--trigger system_stats HOST_NAME=\"Alice's Mac\" CPU_USAGE=\"10%\"",
      payload_expected, sizeof(payload_expected));

  const char escaped_expected[] = "HOST_NAME=Alice's \"Mac\" \\ drive\0";
  assert_tokens("HOST_NAME=\"Alice's \\\"Mac\\\" \\\\ drive\"",
                escaped_expected, sizeof(escaped_expected));
}

static void reply_to(mach_port_t port) {
  struct mach_message response = {0};
  response.header.msgh_remote_port = port;
  response.header.msgh_bits =
      MACH_MSGH_BITS_SET(MACH_MSG_TYPE_COPY_SEND, 0, 0, MACH_MSGH_BITS_COMPLEX);
  response.header.msgh_size = sizeof(response);
  response.msgh_descriptor_count = 1;
  response.descriptor.address = "";
  response.descriptor.size = 1;
  response.descriptor.copy = MACH_MSG_VIRTUAL_COPY;
  response.descriptor.type = MACH_MSG_OOL_DESCRIPTOR;
  (void)mach_msg(&response.header, MACH_SEND_MSG | MACH_SEND_TIMEOUT,
                 sizeof(response), 0, MACH_PORT_NULL, 0, MACH_PORT_NULL);
}

static void *receive_and_reply(void *argument) {
  bool delay_reply = *(bool *)argument;
  struct mach_buffer buffer;
  assert(mach_receive_message(private_port, &buffer, false));
  assert(buffer.message.descriptor.address != NULL);
  const char *message = buffer.message.descriptor.address;
  size_t message_len = buffer.message.descriptor.size;
  assert(message_len >= 2 && message[message_len - 1] == '\0' &&
         message[message_len - 2] == '\0');
  ++received_messages;
  mach_port_t response_port = buffer.message.header.msgh_remote_port;

  if (delay_reply) {
    usleep(150000);
  }
  reply_to(response_port);
  mach_msg_destroy(&buffer.message.header);
  return NULL;
}

static void create_private_port(void) {
  assert(mach_port_allocate(mach_task_self(), MACH_PORT_RIGHT_RECEIVE,
                            &private_port) == KERN_SUCCESS);
  assert(mach_port_insert_right(mach_task_self(), private_port, private_port,
                                MACH_MSG_TYPE_MAKE_SEND) == KERN_SUCCESS);
}

static void test_delayed_reply_does_not_resend(void) {
  create_private_port();
  bool delay_reply = true;
  pthread_t thread;
  assert(pthread_create(&thread, NULL, receive_and_reply, &delay_reply) == 0);
  char *response = sketchybar("--trigger system_stats", "private-probe");
  assert(response == NULL);
  assert(pthread_join(thread, NULL) == 0);
  assert(received_messages == 1);
  struct mach_buffer unexpected_request = {0};
  mach_msg_return_t receive_result = mach_msg(
      &unexpected_request.message.header, MACH_RCV_MSG | MACH_RCV_TIMEOUT, 0,
      sizeof(unexpected_request), private_port, 10, MACH_PORT_NULL);
  assert(receive_result == MACH_RCV_TIMED_OUT);
  cleanup_sketchybar();
}

static void test_full_queue_send_is_bounded(void) {
  create_private_port();
  mach_msg_header_t message = {0};
  message.msgh_remote_port = private_port;
  message.msgh_bits = MACH_MSGH_BITS(MACH_MSG_TYPE_COPY_SEND, 0);
  message.msgh_size = sizeof(message);
  size_t queued = 0;
  while (mach_msg(&message, MACH_SEND_MSG | MACH_SEND_TIMEOUT, sizeof(message),
                  0, MACH_PORT_NULL, 0, MACH_PORT_NULL) == MACH_MSG_SUCCESS) {
    ++queued;
  }
  assert(queued > 0);

  char *response = NULL;
  struct timespec start;
  struct timespec end;
  assert(clock_gettime(CLOCK_MONOTONIC, &start) == 0);
  enum mach_send_status status = mach_send_message(
      private_port, "--trigger\0system_stats\0\0", 24, &response);
  assert(clock_gettime(CLOCK_MONOTONIC, &end) == 0);

  double elapsed = (double)(end.tv_sec - start.tv_sec) +
                   (double)(end.tv_nsec - start.tv_nsec) / 1e9;
  assert(status == MACH_SEND_FAILED);
  assert(response == NULL);
  assert(elapsed >= 0.5 && elapsed < 5.0);
}

int main(int argc, char **argv) {
  assert(argc == 2);
  if (strcmp(argv[1], "tokens") == 0) {
    test_tokenizer();
  } else if (strcmp(argv[1], "delayed-reply") == 0) {
    test_delayed_reply_does_not_resend();
  } else if (strcmp(argv[1], "full-queue") == 0) {
    test_full_queue_send_is_bounded();
  } else {
    return 2;
  }
  return 0;
}
