#include "sketchybar.h"

#include <bootstrap.h>
#include <mach/mach.h>
#include <mach/message.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SKETCHYBAR_SEND_TIMEOUT_MS 1000
#define SKETCHYBAR_RESPONSE_TIMEOUT_MS 100
#define SKETCHYBAR_SERVICE_NAME_SIZE 256U

struct mach_message {
  mach_msg_header_t header;
  mach_msg_size_t msgh_descriptor_count;
  mach_msg_ool_descriptor_t descriptor;
};

struct mach_buffer {
  struct mach_message message;
  mach_msg_trailer_t trailer;
};

static mach_port_t g_mach_port = MACH_PORT_NULL;
static char g_service_name[SKETCHYBAR_SERVICE_NAME_SIZE];
static pthread_mutex_t g_port_mutex = PTHREAD_MUTEX_INITIALIZER;

static bool make_service_name(const char *bar_name,
                              char service_name[SKETCHYBAR_SERVICE_NAME_SIZE]) {
  if (bar_name == NULL || bar_name[0] == '\0') {
    return false;
  }

  int length = snprintf(service_name, SKETCHYBAR_SERVICE_NAME_SIZE,
                        "git.felix.%s", bar_name);
  return length >= 0 && (size_t)length < SKETCHYBAR_SERVICE_NAME_SIZE;
}

static mach_port_t mach_get_bs_port(const char *service_name) {
  mach_port_t bootstrap_port = MACH_PORT_NULL;
  if (task_get_special_port(mach_task_self(), TASK_BOOTSTRAP_PORT,
                            &bootstrap_port) != KERN_SUCCESS) {
    return MACH_PORT_NULL;
  }

  mach_port_t port = MACH_PORT_NULL;
  kern_return_t result = bootstrap_look_up(bootstrap_port, service_name, &port);
  mach_port_deallocate(mach_task_self(), bootstrap_port);

  return result == KERN_SUCCESS ? port : MACH_PORT_NULL;
}

static void release_cached_port_locked(void) {
  if (g_mach_port != MACH_PORT_NULL) {
    mach_port_deallocate(mach_task_self(), g_mach_port);
    g_mach_port = MACH_PORT_NULL;
  }
  g_service_name[0] = '\0';
}

static mach_port_t cache_service_port_locked(const char *service_name) {
  if (g_mach_port != MACH_PORT_NULL &&
      strcmp(g_service_name, service_name) == 0) {
    return g_mach_port;
  }

  release_cached_port_locked();
  mach_port_t port = mach_get_bs_port(service_name);
  if (port != MACH_PORT_NULL) {
    g_mach_port = port;
    (void)snprintf(g_service_name, sizeof(g_service_name), "%s", service_name);
  }
  return g_mach_port;
}

static bool mach_receive_message(mach_port_t port, struct mach_buffer *buffer,
                                 bool timeout) {
  *buffer = (struct mach_buffer){0};
  mach_msg_return_t result;
  if (timeout) {
    result = mach_msg(&buffer->message.header, MACH_RCV_MSG | MACH_RCV_TIMEOUT,
                      0, (mach_msg_size_t)sizeof(*buffer), port,
                      SKETCHYBAR_RESPONSE_TIMEOUT_MS, MACH_PORT_NULL);
  } else {
    result = mach_msg(&buffer->message.header, MACH_RCV_MSG, 0,
                      (mach_msg_size_t)sizeof(*buffer), port,
                      MACH_MSG_TIMEOUT_NONE, MACH_PORT_NULL);
  }

  if (result != MACH_MSG_SUCCESS) {
    buffer->message.descriptor.address = NULL;
    return false;
  }
  return true;
}

static void destroy_pseudo_received_send_message(struct mach_message *message,
                                                 const char *owned_payload) {
  mach_msg_header_t *header = &message->header;
  mach_port_t local_port = header->msgh_local_port;
  mach_msg_type_name_t local_disposition =
      MACH_MSGH_BITS_LOCAL(header->msgh_bits);

  if (local_port != MACH_PORT_NULL && local_port != MACH_PORT_DEAD) {
    switch (local_disposition) {
    case MACH_MSG_TYPE_MOVE_SEND:
    case MACH_MSG_TYPE_MOVE_SEND_ONCE:
      mach_port_deallocate(mach_task_self(), local_port);
      header->msgh_local_port = MACH_PORT_NULL;
      break;
    default:
      break;
    }
  }

  /* The pseudo-receive maps the OOL copy back into this task. Keep the
   * caller's original allocation if the returned descriptor aliases it. */
  if (message->descriptor.address == owned_payload) {
    message->descriptor.deallocate = false;
  }
  mach_msg_destroy(header);
}

static enum sketchybar_send_status mach_send_message(mach_port_t port,
                                                     const char *message,
                                                     uint32_t length,
                                                     char **response) {
  if (message == NULL || port == MACH_PORT_NULL) {
    return SKETCHYBAR_SEND_FAILED;
  }

  mach_port_t task = mach_task_self();
  mach_port_t response_port = MACH_PORT_NULL;
  if (mach_port_allocate(task, MACH_PORT_RIGHT_RECEIVE, &response_port) !=
      KERN_SUCCESS) {
    return SKETCHYBAR_SEND_FAILED;
  }

  struct mach_message msg = {0};
  msg.header.msgh_remote_port = port;
  msg.header.msgh_local_port = response_port;
  msg.header.msgh_id = response_port;
  msg.header.msgh_bits =
      MACH_MSGH_BITS_SET(MACH_MSG_TYPE_COPY_SEND, MACH_MSG_TYPE_MAKE_SEND, 0,
                         MACH_MSGH_BITS_COMPLEX);
  msg.header.msgh_size = (mach_msg_size_t)sizeof(msg);
  msg.msgh_descriptor_count = 1;
  msg.descriptor.address = (void *)message;
  msg.descriptor.size = length;
  msg.descriptor.copy = MACH_MSG_VIRTUAL_COPY;
  msg.descriptor.deallocate = false;
  msg.descriptor.type = MACH_MSG_OOL_DESCRIPTOR;

  mach_msg_return_t send_result =
      mach_msg(&msg.header, MACH_SEND_MSG | MACH_SEND_TIMEOUT,
               (mach_msg_size_t)sizeof(msg), 0, MACH_PORT_NULL,
               SKETCHYBAR_SEND_TIMEOUT_MS, MACH_PORT_NULL);
  if (send_result != MACH_MSG_SUCCESS) {
    mach_msg_return_t send_error = send_result & ~MACH_MSG_MASK;
    if (send_error == MACH_SEND_TIMED_OUT ||
        send_error == MACH_SEND_INTERRUPTED) {
      destroy_pseudo_received_send_message(&msg, message);
    }
    mach_port_mod_refs(task, response_port, MACH_PORT_RIGHT_RECEIVE, -1);
    return SKETCHYBAR_SEND_FAILED;
  }

  struct mach_buffer buffer = {0};
  bool received = mach_receive_message(response_port, &buffer, true);
  if (received && response != NULL &&
      buffer.message.descriptor.address != NULL) {
    size_t result_length = buffer.message.descriptor.size;
    if (result_length < SIZE_MAX) {
      char *copy = malloc(result_length + 1);
      if (copy != NULL) {
        memcpy(copy, buffer.message.descriptor.address, result_length);
        copy[result_length] = '\0';
        *response = copy;
      }
    }
  }

  if (received) {
    mach_msg_destroy(&buffer.message.header);
  }
  mach_port_mod_refs(task, response_port, MACH_PORT_RIGHT_RECEIVE, -1);

  return received ? SKETCHYBAR_SEND_ACKNOWLEDGED : SKETCHYBAR_SENT_NO_ACK;
}

static size_t tokenize_sketchybar_message(const char *message, char *output) {
  char quote = '\0';
  bool token_started = false;
  size_t output_len = 0;

  for (size_t i = 0; message[i] != '\0'; ++i) {
    char current = message[i];

    if (current == '\\' && message[i + 1] != '\0') {
      char next = message[i + 1];
      if ((quote && (next == quote || next == '\\')) ||
          (!quote && (next == '\\' || next == ' ' || next == '\t' ||
                      next == '"' || next == '\''))) {
        output[output_len++] = next;
        token_started = true;
        ++i;
        continue;
      }
      output[output_len++] = current;
      token_started = true;
      continue;
    }

    if (quote) {
      if (current == quote) {
        quote = '\0';
      } else {
        output[output_len++] = current;
      }
      token_started = true;
      continue;
    }

    if (current == '"' || current == '\'') {
      quote = current;
      token_started = true;
    } else if (current == ' ' || current == '\t' || current == '\n') {
      if (token_started) {
        output[output_len++] = '\0';
        token_started = false;
      }
    } else {
      output[output_len++] = current;
      token_started = true;
    }
  }

  if (token_started) {
    output[output_len++] = '\0';
  } else if (output_len == 0) {
    output[output_len++] = '\0';
  }
  output[output_len++] = '\0';
  return output_len;
}

enum sketchybar_send_status
sketchybar_send(const char *message, const char *bar_name, char **response) {
  if (response != NULL) {
    *response = NULL;
  }
  if (message == NULL) {
    return SKETCHYBAR_SEND_FAILED;
  }

  char service_name[SKETCHYBAR_SERVICE_NAME_SIZE];
  if (!make_service_name(bar_name, service_name)) {
    return SKETCHYBAR_SEND_FAILED;
  }

  size_t message_length = strlen(message);
  if (message_length > SIZE_MAX - 2) {
    return SKETCHYBAR_SEND_FAILED;
  }

  char *formatted_message = calloc(message_length + 2, sizeof(char));
  if (formatted_message == NULL) {
    return SKETCHYBAR_SEND_FAILED;
  }

  size_t formatted_length =
      tokenize_sketchybar_message(message, formatted_message);
  if (formatted_length > UINT32_MAX) {
    free(formatted_message);
    return SKETCHYBAR_SEND_FAILED;
  }

  pthread_mutex_lock(&g_port_mutex);
  mach_port_t port = cache_service_port_locked(service_name);
  enum sketchybar_send_status status = mach_send_message(
      port, formatted_message, (uint32_t)formatted_length, response);
  if (status == SKETCHYBAR_SEND_FAILED) {
    release_cached_port_locked();
    port = cache_service_port_locked(service_name);
    status = mach_send_message(port, formatted_message,
                               (uint32_t)formatted_length, response);
    if (status == SKETCHYBAR_SEND_FAILED) {
      release_cached_port_locked();
    }
  }
  pthread_mutex_unlock(&g_port_mutex);

  free(formatted_message);
  return status;
}

char *sketchybar(const char *message, const char *bar_name) {
  char *response = NULL;
  (void)sketchybar_send(message, bar_name, &response);
  return response;
}

void free_sketchybar_response(char *response) { free(response); }

bool refresh_sketchybar_port(const char *bar_name) {
  char service_name[SKETCHYBAR_SERVICE_NAME_SIZE];
  if (!make_service_name(bar_name, service_name)) {
    return false;
  }

  pthread_mutex_lock(&g_port_mutex);
  release_cached_port_locked();
  bool success = cache_service_port_locked(service_name) != MACH_PORT_NULL;
  pthread_mutex_unlock(&g_port_mutex);
  return success;
}

void cleanup_sketchybar(void) {
  pthread_mutex_lock(&g_port_mutex);
  release_cached_port_locked();
  pthread_mutex_unlock(&g_port_mutex);
}
