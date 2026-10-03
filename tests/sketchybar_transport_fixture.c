#define _POSIX_C_SOURCE 200809L

#include <assert.h>
#include <bootstrap.h>
#include <mach/mach.h>
#include <mach/message.h>
#include <pthread.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "../include/sketchybar.h"

struct transport_message {
  mach_msg_header_t header;
  mach_msg_size_t descriptor_count;
  mach_msg_ool_descriptor_t descriptor;
};

struct transport_buffer {
  struct transport_message message;
  mach_msg_trailer_t trailer;
};

struct receiver_args {
  mach_port_t port;
  const char *expected;
  size_t expected_len;
  const char *reply;
  unsigned int delay_ms;
  int received;
};

static mach_port_t alpha_port = MACH_PORT_NULL;
static mach_port_t beta_port = MACH_PORT_NULL;
static unsigned int lookup_count;

kern_return_t probe_lookup(mach_port_t bootstrap_port, const char *name,
                           mach_port_t *port) {
  (void)bootstrap_port;
  ++lookup_count;

  mach_port_t selected = MACH_PORT_NULL;
  if (strcmp(name, "git.felix.alpha") == 0) {
    selected = alpha_port;
  } else if (strcmp(name, "git.felix.beta") == 0) {
    selected = beta_port;
  }
  if (selected == MACH_PORT_NULL) {
    return BOOTSTRAP_UNKNOWN_SERVICE;
  }

  kern_return_t result =
      mach_port_mod_refs(mach_task_self(), selected, MACH_PORT_RIGHT_SEND, 1);
  if (result == KERN_SUCCESS) {
    *port = selected;
  }
  return result;
}

static void make_private_port(mach_port_t *port) {
  assert(mach_port_allocate(mach_task_self(), MACH_PORT_RIGHT_RECEIVE, port) ==
         KERN_SUCCESS);
  assert(mach_port_insert_right(mach_task_self(), *port, *port,
                                MACH_MSG_TYPE_MAKE_SEND) == KERN_SUCCESS);
}

static void pause_ms(unsigned int milliseconds) {
  struct timespec remaining = {.tv_sec = milliseconds / 1000,
                               .tv_nsec =
                                   (long)(milliseconds % 1000) * 1000000};
  while (nanosleep(&remaining, &remaining) != 0) {
  }
}

static void reply_to(mach_port_t port, const char *text, bool may_be_late) {
  struct transport_message response = {0};
  response.header.msgh_remote_port = port;
  response.header.msgh_bits =
      MACH_MSGH_BITS_SET(MACH_MSG_TYPE_COPY_SEND, 0, 0, MACH_MSGH_BITS_COMPLEX);
  response.header.msgh_size = sizeof(response);
  response.descriptor_count = 1;
  response.descriptor.address = (void *)text;
  response.descriptor.size = (mach_msg_size_t)(strlen(text) + 1);
  response.descriptor.copy = MACH_MSG_VIRTUAL_COPY;
  response.descriptor.deallocate = false;
  response.descriptor.type = MACH_MSG_OOL_DESCRIPTOR;
  mach_msg_return_t result =
      mach_msg(&response.header, MACH_SEND_MSG | MACH_SEND_TIMEOUT,
               sizeof(response), 0, MACH_PORT_NULL, 1000, MACH_PORT_NULL);
  assert(result == MACH_MSG_SUCCESS ||
         (may_be_late && result == MACH_SEND_INVALID_DEST));
}

static void *receive_and_reply(void *opaque) {
  struct receiver_args *args = opaque;
  struct transport_buffer buffer = {0};
  mach_msg_return_t result =
      mach_msg(&buffer.message.header, MACH_RCV_MSG, 0, sizeof(buffer),
               args->port, MACH_MSG_TIMEOUT_NONE, MACH_PORT_NULL);
  assert(result == MACH_MSG_SUCCESS);
  assert(buffer.message.descriptor_count == 1);
  assert(buffer.message.descriptor.address != NULL);
  assert(buffer.message.descriptor.size == args->expected_len);
  assert(memcmp(buffer.message.descriptor.address, args->expected,
                args->expected_len) == 0);
  ++args->received;

  if (args->delay_ms != 0) {
    pause_ms(args->delay_ms);
  }
  reply_to(buffer.message.header.msgh_remote_port, args->reply,
           args->delay_ms != 0);
  mach_msg_destroy(&buffer.message.header);
  return NULL;
}

static pthread_t start_receiver(struct receiver_args *args) {
  pthread_t thread;
  args->received = 0;
  assert(pthread_create(&thread, NULL, receive_and_reply, args) == 0);
  return thread;
}

static void join_receiver(pthread_t thread, struct receiver_args *args) {
  assert(pthread_join(thread, NULL) == 0);
  assert(args->received == 1);
}

static void assert_no_queued_message(mach_port_t port) {
  struct transport_buffer unexpected = {0};
  mach_msg_return_t result =
      mach_msg(&unexpected.message.header, MACH_RCV_MSG | MACH_RCV_TIMEOUT, 0,
               sizeof(unexpected), port, 10, MACH_PORT_NULL);
  assert(result == MACH_RCV_TIMED_OUT);
}

static size_t count_dead_names(void) {
  mach_port_name_array_t names = NULL;
  mach_port_type_array_t types = NULL;
  mach_msg_type_number_t name_count = 0;
  mach_msg_type_number_t type_count = 0;
  assert(mach_port_names(mach_task_self(), &names, &name_count, &types,
                         &type_count) == KERN_SUCCESS);
  assert(name_count == type_count);

  size_t dead_names = 0;
  for (mach_msg_type_number_t i = 0; i < type_count; ++i) {
    if ((types[i] & MACH_PORT_TYPE_DEAD_NAME) != 0) {
      ++dead_names;
    }
  }
  if (names != NULL) {
    assert(vm_deallocate(mach_task_self(), (vm_address_t)names,
                         (vm_size_t)name_count * sizeof(*names)) ==
           KERN_SUCCESS);
  }
  if (types != NULL) {
    assert(vm_deallocate(mach_task_self(), (vm_address_t)types,
                         (vm_size_t)type_count * sizeof(*types)) ==
           KERN_SUCCESS);
  }
  return dead_names;
}

static void test_token_round_trips(void) {
  make_private_port(&alpha_port);
  struct receiver_args cases[] = {
      {.port = alpha_port,
       .expected = "\0",
       .expected_len = 2,
       .reply = "empty"},
      {.port = alpha_port,
       .expected = "one\0\0",
       .expected_len = 5,
       .reply = "plain"},
      {.port = alpha_port,
       .expected =
           "--trigger\0system_stats\0HOST_NAME=Alice's Mac\0CPU_USAGE=10%\0",
       .expected_len = sizeof(
           "--trigger\0system_stats\0HOST_NAME=Alice's Mac\0CPU_USAGE=10%\0"),
       .reply = "payload"},
      {.port = alpha_port,
       .expected = "HOST_NAME=Alice's \"Mac\" \\ drive\0",
       .expected_len = sizeof("HOST_NAME=Alice's \"Mac\" \\ drive\0"),
       .reply = "escaped"},
  };
  const char *commands[] = {
      " \t",
      "one  \t",
      "--trigger system_stats HOST_NAME=\"Alice's Mac\" CPU_USAGE=\"10%\"",
      "HOST_NAME=\"Alice's \\\"Mac\\\" \\\\ drive\"",
  };

  for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); ++i) {
    pthread_t receiver = start_receiver(&cases[i]);
    char *response = NULL;
    enum sketchybar_send_status status =
        sketchybar_send(commands[i], "alpha", i == 2 ? &response : NULL);
    assert(status == SKETCHYBAR_SEND_ACKNOWLEDGED);
    if (i == 2) {
      assert(response != NULL);
      assert(strcmp(response, "payload") == 0);
      free_sketchybar_response(response);
    }
    join_receiver(receiver, &cases[i]);
  }
  cleanup_sketchybar();
}

static void test_delayed_ack_does_not_resend(void) {
  make_private_port(&alpha_port);
  const char expected[] = "--trigger\0system_stats\0";
  struct receiver_args args = {.port = alpha_port,
                               .expected = expected,
                               .expected_len = sizeof(expected),
                               .reply = "late",
                               .delay_ms = 150};
  pthread_t receiver = start_receiver(&args);

  char *response = NULL;
  enum sketchybar_send_status status =
      sketchybar_send("--trigger system_stats", "alpha", &response);
  assert(status == SKETCHYBAR_SENT_NO_ACK);
  assert(response == NULL);
  join_receiver(receiver, &args);
  assert_no_queued_message(alpha_port);

  unsigned int lookups_after_no_ack = lookup_count;
  const char followup_expected[] = "followup\0";
  struct receiver_args followup = {.port = alpha_port,
                                   .expected = followup_expected,
                                   .expected_len = sizeof(followup_expected),
                                   .reply = "followup-ack"};
  receiver = start_receiver(&followup);
  assert(sketchybar_send("followup", "alpha", NULL) ==
         SKETCHYBAR_SEND_ACKNOWLEDGED);
  join_receiver(receiver, &followup);
  assert(lookup_count == lookups_after_no_ack);
  cleanup_sketchybar();
}

static void send_routed_message(mach_port_t port, const char *bar_name,
                                const char *value) {
  char expected[64];
  const size_t value_len = strlen(value);
  assert(value_len + sizeof("route\0\0") <= sizeof(expected));
  memcpy(expected, "route\0", sizeof("route"));
  memcpy(expected + sizeof("route"), value, value_len);
  expected[sizeof("route") + value_len] = '\0';
  expected[sizeof("route") + value_len + 1] = '\0';
  struct receiver_args args = {.port = port,
                               .expected = expected,
                               .expected_len = sizeof("route") + value_len + 2,
                               .reply = "routed"};
  pthread_t receiver = start_receiver(&args);
  char command[64];
  int command_len = snprintf(command, sizeof(command), "route %s", value);
  assert(command_len > 0 && (size_t)command_len < sizeof(command));
  assert(sketchybar_send(command, bar_name, NULL) ==
         SKETCHYBAR_SEND_ACKNOWLEDGED);
  join_receiver(receiver, &args);
}

static void test_named_endpoints_and_refresh(void) {
  make_private_port(&alpha_port);
  make_private_port(&beta_port);

  send_routed_message(alpha_port, "alpha", "alpha-one");
  send_routed_message(beta_port, "beta", "beta-one");
  send_routed_message(alpha_port, "alpha", "alpha-two");

  assert(refresh_sketchybar_port("beta"));
  send_routed_message(alpha_port, "alpha", "alpha-after-beta-refresh");
  assert(refresh_sketchybar_port("alpha"));
  send_routed_message(beta_port, "beta", "beta-after-alpha-refresh");
  cleanup_sketchybar();
}

static void test_oversized_name_does_not_lookup(void) {
  make_private_port(&alpha_port);
  char name[512];
  memset(name, 'x', sizeof(name) - 1);
  name[sizeof(name) - 1] = '\0';
  unsigned int before = lookup_count;
  assert(sketchybar_send("--trigger system_stats", name, NULL) ==
         SKETCHYBAR_SEND_FAILED);
  assert(lookup_count == before);
  assert_no_queued_message(alpha_port);
  cleanup_sketchybar();
}

static void test_full_queue_send_is_bounded(void) {
  make_private_port(&alpha_port);
  mach_msg_header_t message = {0};
  message.msgh_remote_port = alpha_port;
  message.msgh_bits = MACH_MSGH_BITS(MACH_MSG_TYPE_COPY_SEND, 0);
  message.msgh_size = sizeof(message);
  size_t queued = 0;
  while (mach_msg(&message, MACH_SEND_MSG | MACH_SEND_TIMEOUT, sizeof(message),
                  0, MACH_PORT_NULL, 0, MACH_PORT_NULL) == MACH_MSG_SUCCESS) {
    ++queued;
  }
  assert(queued > 0);
  mach_msg_destroy(&message);
  mach_port_urefs_t baseline_send_refs = 0;
  assert(mach_port_get_refs(mach_task_self(), alpha_port, MACH_PORT_RIGHT_SEND,
                            &baseline_send_refs) == KERN_SUCCESS);
  size_t dead_names_before = count_dead_names();

  struct timespec start;
  struct timespec end;
  assert(clock_gettime(CLOCK_MONOTONIC, &start) == 0);
  enum sketchybar_send_status status =
      sketchybar_send("--trigger system_stats", "alpha", NULL);
  assert(clock_gettime(CLOCK_MONOTONIC, &end) == 0);
  double elapsed = (double)(end.tv_sec - start.tv_sec) +
                   (double)(end.tv_nsec - start.tv_nsec) / 1e9;
  assert(status == SKETCHYBAR_SEND_FAILED);
  assert(elapsed >= 0.5 && elapsed < 5.0);

  mach_port_urefs_t refs_after_failure = 0;
  assert(mach_port_get_refs(mach_task_self(), alpha_port, MACH_PORT_RIGHT_SEND,
                            &refs_after_failure) == KERN_SUCCESS);
  if (refs_after_failure != baseline_send_refs) {
    fprintf(stderr, "private port send refs changed from %u to %u\n",
            baseline_send_refs, refs_after_failure);
  }
  assert(refs_after_failure == baseline_send_refs);
  assert(count_dead_names() == dead_names_before);

  size_t drained = 0;
  for (;;) {
    struct transport_buffer queued_message = {0};
    mach_msg_return_t result = mach_msg(
        &queued_message.message.header, MACH_RCV_MSG | MACH_RCV_TIMEOUT, 0,
        sizeof(queued_message), alpha_port, 10, MACH_PORT_NULL);
    if (result == MACH_RCV_TIMED_OUT) {
      break;
    }
    assert(result == MACH_MSG_SUCCESS);
    mach_msg_destroy(&queued_message.message.header);
    ++drained;
  }
  assert(drained == queued);

  unsigned int lookups_before_recovery = lookup_count;
  send_routed_message(alpha_port, "alpha", "after-full-queue");
  assert(lookup_count == lookups_before_recovery + 1);
  cleanup_sketchybar();
}

static void test_null_arguments_are_safe(void) {
  unsigned int before = lookup_count;
  assert(sketchybar_send(NULL, "alpha", NULL) == SKETCHYBAR_SEND_FAILED);
  assert(sketchybar_send("--trigger system_stats", NULL, NULL) ==
         SKETCHYBAR_SEND_FAILED);
  assert(sketchybar(NULL, "alpha") == NULL);
  assert(sketchybar("--trigger system_stats", NULL) == NULL);
  assert(lookup_count == before);
}

int main(int argc, char **argv) {
  assert(argc == 2);
  if (strcmp(argv[1], "tokens") == 0) {
    test_token_round_trips();
  } else if (strcmp(argv[1], "delayed-reply") == 0) {
    test_delayed_ack_does_not_resend();
  } else if (strcmp(argv[1], "routing") == 0) {
    test_named_endpoints_and_refresh();
  } else if (strcmp(argv[1], "oversized-name") == 0) {
    test_oversized_name_does_not_lookup();
  } else if (strcmp(argv[1], "full-queue") == 0) {
    test_full_queue_send_is_bounded();
  } else if (strcmp(argv[1], "null-inputs") == 0) {
    test_null_arguments_are_safe();
  } else {
    return 2;
  }
  return 0;
}
