#pragma once

#include <bootstrap.h>
#include <mach/mach.h>
#include <mach/message.h>
#include <pthread.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SKETCHYBAR_SEND_TIMEOUT_MS 1000
#define SKETCHYBAR_RESPONSE_TIMEOUT_MS 100

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
static pthread_mutex_t g_port_mutex = PTHREAD_MUTEX_INITIALIZER;

mach_port_t mach_get_bs_port(char *bar_name) {
  mach_port_name_t task = mach_task_self();

  mach_port_t bs_port;
  if (task_get_special_port(task, TASK_BOOTSTRAP_PORT, &bs_port) !=
      KERN_SUCCESS) {
    return MACH_PORT_NULL;
  }

  char service_name[256]; // Assuming the service name will not exceed 255 chars
  snprintf(service_name, sizeof(service_name), "git.felix.%s", bar_name);

  mach_port_t port;
  kern_return_t result = bootstrap_look_up(bs_port, service_name, &port);
  mach_port_deallocate(task, bs_port);

  return (result == KERN_SUCCESS) ? port : MACH_PORT_NULL;
}

bool mach_receive_message(mach_port_t port, struct mach_buffer *buffer,
                          bool timeout) {
  *buffer = (struct mach_buffer){0};
  mach_msg_return_t msg_return;
  if (timeout)
    msg_return =
        mach_msg(&buffer->message.header, MACH_RCV_MSG | MACH_RCV_TIMEOUT, 0,
                 sizeof(struct mach_buffer), port,
                 SKETCHYBAR_RESPONSE_TIMEOUT_MS, MACH_PORT_NULL);
  else
    msg_return = mach_msg(&buffer->message.header, MACH_RCV_MSG, 0,
                          sizeof(struct mach_buffer), port,
                          MACH_MSG_TIMEOUT_NONE, MACH_PORT_NULL);

  if (msg_return != MACH_MSG_SUCCESS) {
    buffer->message.descriptor.address = NULL;
    return false;
  }
  return true;
}

enum mach_send_status {
  MACH_SEND_FAILED,
  MACH_SEND_ACKNOWLEDGED,
  MACH_SEND_ACK_TIMEOUT,
};

enum mach_send_status mach_send_message(mach_port_t port, const char *message,
                                        uint32_t len, char **response) {
  if (!response) {
    return MACH_SEND_FAILED;
  }
  *response = NULL;
  if (!message || !port) {
    return MACH_SEND_FAILED;
  }

  mach_port_t response_port;
  mach_port_name_t task = mach_task_self();
  if (mach_port_allocate(task, MACH_PORT_RIGHT_RECEIVE, &response_port) !=
      KERN_SUCCESS) {
    return MACH_SEND_FAILED;
  }

  if (mach_port_insert_right(task, response_port, response_port,
                             MACH_MSG_TYPE_MAKE_SEND) != KERN_SUCCESS) {
    mach_port_mod_refs(task, response_port, MACH_PORT_RIGHT_RECEIVE, -1);
    return MACH_SEND_FAILED;
  }

  struct mach_message msg = {0};
  msg.header.msgh_remote_port = port;
  msg.header.msgh_local_port = response_port;
  msg.header.msgh_id = response_port;
  msg.header.msgh_bits =
      MACH_MSGH_BITS_SET(MACH_MSG_TYPE_COPY_SEND, MACH_MSG_TYPE_MAKE_SEND, 0,
                         MACH_MSGH_BITS_COMPLEX);

  msg.header.msgh_size = sizeof(struct mach_message);
  msg.msgh_descriptor_count = 1;
  msg.descriptor.address = (void *)message;
  msg.descriptor.size = len * sizeof(char);
  msg.descriptor.copy = MACH_MSG_VIRTUAL_COPY;
  msg.descriptor.deallocate = false;
  msg.descriptor.type = MACH_MSG_OOL_DESCRIPTOR;

  mach_msg_return_t send_result =
      mach_msg(&msg.header, MACH_SEND_MSG | MACH_SEND_TIMEOUT,
               sizeof(struct mach_message), 0, MACH_PORT_NULL,
               SKETCHYBAR_SEND_TIMEOUT_MS, MACH_PORT_NULL);

  if (send_result != MACH_MSG_SUCCESS) {
    mach_port_mod_refs(task, response_port, MACH_PORT_RIGHT_RECEIVE, -1);
    mach_port_deallocate(task, response_port);
    return MACH_SEND_FAILED;
  }

  struct mach_buffer buffer = {0};
  bool received = mach_receive_message(response_port, &buffer, true);

  if (received && buffer.message.descriptor.address) {
    size_t result_len = buffer.message.descriptor.size;
    *response = (char *)malloc(result_len + 1);
    if (*response) {
      memcpy(*response, buffer.message.descriptor.address, result_len);
      (*response)[result_len] = '\0';
    }
  }

  if (received) {
    mach_msg_destroy(&buffer.message.header);
  }
  mach_port_mod_refs(task, response_port, MACH_PORT_RIGHT_RECEIVE, -1);
  mach_port_deallocate(task, response_port);

  return received ? MACH_SEND_ACKNOWLEDGED : MACH_SEND_ACK_TIMEOUT;
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

char *sketchybar(const char *message, const char *bar_name) {
  if (!message || !bar_name) {
    return NULL;
  }

  size_t message_length = strlen(message);
  char *formatted_message = (char *)calloc(message_length + 2, sizeof(char));
  if (!formatted_message) {
    return NULL;
  }

  size_t formatted_length =
      tokenize_sketchybar_message(message, formatted_message);

  pthread_mutex_lock(&g_port_mutex);
  if (g_mach_port == MACH_PORT_NULL) {
    g_mach_port = mach_get_bs_port((char *)bar_name);
  }

  char *response = NULL;
  enum mach_send_status status = mach_send_message(
      g_mach_port, formatted_message, (uint32_t)formatted_length, &response);
  if (status == MACH_SEND_FAILED) {
    if (g_mach_port != MACH_PORT_NULL) {
      mach_port_deallocate(mach_task_self(), g_mach_port);
      g_mach_port = MACH_PORT_NULL;
    }
    g_mach_port = mach_get_bs_port((char *)bar_name);
    (void)mach_send_message(g_mach_port, formatted_message,
                            (uint32_t)formatted_length, &response);
  }
  pthread_mutex_unlock(&g_port_mutex);

  free(formatted_message);

  return response;
}

bool refresh_sketchybar_port(const char *bar_name) {
  pthread_mutex_lock(&g_port_mutex);
  if (g_mach_port != MACH_PORT_NULL) {
    mach_port_deallocate(mach_task_self(), g_mach_port);
  }
  g_mach_port = mach_get_bs_port((char *)bar_name);
  bool success = (g_mach_port != MACH_PORT_NULL);
  pthread_mutex_unlock(&g_port_mutex);
  return success;
}

void free_sketchybar_response(char *response) { free(response); }

void cleanup_sketchybar() {
  pthread_mutex_lock(&g_port_mutex);
  if (g_mach_port != MACH_PORT_NULL) {
    mach_port_deallocate(mach_task_self(), g_mach_port);
    g_mach_port = MACH_PORT_NULL;
  }
  pthread_mutex_unlock(&g_port_mutex);
}
