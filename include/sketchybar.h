#pragma once

#include <bootstrap.h>
#include <mach/mach.h>
#include <mach/message.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>

typedef char *env;

#define MACH_HANDLER(name) void name(env env)
typedef MACH_HANDLER(mach_handler);

struct mach_message {
  mach_msg_header_t header;
  mach_msg_size_t msgh_descriptor_count;
  mach_msg_ool_descriptor_t descriptor;
};

struct mach_buffer {
  struct mach_message message;
  mach_msg_trailer_t trailer;
};

struct mach_server {
  bool is_running;
  mach_port_name_t task;
  mach_port_t port;
  mach_port_t bs_port;

  pthread_t thread;
  mach_handler *handler;
};

void deallocate_mach_port(mach_port_t port) {
  if (port != MACH_PORT_NULL) {
    mach_port_deallocate(mach_task_self(), port);
  }
}

static struct mach_server g_mach_server;
static mach_port_t g_mach_port = 0;

char *env_get_value_for_key(env env, char *key) {
  uint32_t caret = 0;
  for (;;) {
    if (!env[caret])
      break;
    if (strcmp(&env[caret], key) == 0)
      return &env[caret + strlen(&env[caret]) + 1];

    caret +=
        strlen(&env[caret]) + strlen(&env[caret + strlen(&env[caret]) + 1]) + 2;
  }
  return (char *)"";
}

mach_port_t mach_get_bs_port(char *bar_name) {
  mach_port_name_t task = mach_task_self();

  mach_port_t bs_port;
  if (task_get_special_port(task, TASK_BOOTSTRAP_PORT, &bs_port) !=
      KERN_SUCCESS) {
    return 0;
  }

  char service_name[256]; // Assuming the service name will not exceed 255 chars
  snprintf(service_name, sizeof(service_name), "git.felix.%s", bar_name);

  mach_port_t port;
  if (bootstrap_look_up(bs_port, service_name, &port) != KERN_SUCCESS) {
    deallocate_mach_port(bs_port);
    return 0;
  }

  deallocate_mach_port(bs_port);
  return port;
}

void mach_receive_message(mach_port_t port, struct mach_buffer *buffer,
                          bool timeout) {
  *buffer = (struct mach_buffer){0};
  mach_msg_return_t msg_return;
  if (timeout)
    msg_return =
        mach_msg(&buffer->message.header, MACH_RCV_MSG | MACH_RCV_TIMEOUT, 0,
                 sizeof(struct mach_buffer), port, 100, MACH_PORT_NULL);
  else
    msg_return = mach_msg(&buffer->message.header, MACH_RCV_MSG, 0,
                          sizeof(struct mach_buffer), port,
                          MACH_MSG_TIMEOUT_NONE, MACH_PORT_NULL);

  if (msg_return != MACH_MSG_SUCCESS) {
    buffer->message.descriptor.address = NULL;
  }
}

void debug_log(const char *func, const char *message) {
  fprintf(stderr, "DEBUG [%s]: %s\n", func, message);
}

char *mach_send_message(mach_port_t port, const char *message, uint32_t len) {
  if (!message || !port) {
    debug_log(__func__, "Null message or port");
    return NULL;
  }

  if (!message || !port) {
    return NULL;
  }

  mach_port_t response_port;
  mach_port_name_t task = mach_task_self();
  if (mach_port_allocate(task, MACH_PORT_RIGHT_RECEIVE, &response_port) !=
      KERN_SUCCESS) {
    return NULL;
  }

  if (mach_port_insert_right(task, response_port, response_port,
                             MACH_MSG_TYPE_MAKE_SEND) != KERN_SUCCESS) {
    mach_port_deallocate(task, response_port);
    return NULL;
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
      mach_msg(&msg.header, MACH_SEND_MSG, sizeof(struct mach_message), 0,
               MACH_PORT_NULL, MACH_MSG_TIMEOUT_NONE, MACH_PORT_NULL);

  if (send_result != MACH_MSG_SUCCESS) {
    mach_port_deallocate(task, response_port);
    return NULL;
  }

  struct mach_buffer buffer = {0};
  mach_receive_message(response_port, &buffer, true);

  char *result = NULL;
  if (buffer.message.descriptor.address) {
    size_t result_len = buffer.message.descriptor.size;
    result = (char *)malloc(result_len + 1);
    if (result) {
      memcpy(result, buffer.message.descriptor.address, result_len);
      result[result_len] = '\0';
    } else {
      debug_log(__func__, "Failed to allocate memory for result");
    }
  } else {
    debug_log(__func__, "No descriptor address");
  }

  mach_msg_destroy(&buffer.message.header);
  mach_port_deallocate(task, response_port);

  return result;
}

#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
bool mach_server_begin(struct mach_server *mach_server, mach_handler handler,
                       char *bootstrap_name) {
  mach_server->task = mach_task_self();

  if (mach_port_allocate(mach_server->task, MACH_PORT_RIGHT_RECEIVE,
                         &mach_server->port) != KERN_SUCCESS) {
    return false;
  }

  if (mach_port_insert_right(mach_server->task, mach_server->port,
                             mach_server->port,
                             MACH_MSG_TYPE_MAKE_SEND) != KERN_SUCCESS) {
    return false;
  }

  if (task_get_special_port(mach_server->task, TASK_BOOTSTRAP_PORT,
                            &mach_server->bs_port) != KERN_SUCCESS) {
    return false;
  }

  if (bootstrap_register(mach_server->bs_port, bootstrap_name,
                         mach_server->port) != KERN_SUCCESS) {
    return false;
  }

  mach_server->handler = handler;
  mach_server->is_running = true;
  struct mach_buffer buffer;
  while (mach_server->is_running) {
    mach_receive_message(mach_server->port, &buffer, false);
    mach_server->handler((env)buffer.message.descriptor.address);
    mach_msg_destroy(&buffer.message.header);
  }

  deallocate_mach_port(mach_server->port);
  deallocate_mach_port(mach_server->bs_port);
  return true;
}
#pragma clang diagnostic pop

char *sketchybar(const char *message, const char *bar_name) {
  if (!message || !bar_name) {
    debug_log(__func__, "Null message or bar_name");
    return strdup("");
  }

  uint32_t message_length = strlen(message) + 1;
  char *formatted_message = (char *)malloc(message_length + 1);
  if (!formatted_message) {
    debug_log(__func__, "Failed to allocate memory for formatted_message");
    return strdup("");
  }

  char quote = '\0';
  uint32_t caret = 0;
  for (uint32_t i = 0; i < message_length; ++i) {
    if (message[i] == '"' || message[i] == '\'') {
      if (quote == message[i])
        quote = '\0';
      else
        quote = message[i];
      continue;
    }
    formatted_message[caret] = message[i];
    if (message[i] == ' ' && !quote)
      formatted_message[caret] = '\0';
    caret++;
  }

  if (caret > 0 && formatted_message[caret] == '\0' &&
      formatted_message[caret - 1] == '\0') {
    caret--;
  }

  formatted_message[caret] = '\0';

  if (!g_mach_port) {
    g_mach_port = mach_get_bs_port((char *)bar_name);
  }

  char *response = NULL;
  if (g_mach_port) {
    response = mach_send_message(g_mach_port, formatted_message, caret + 1);
  } else {
    debug_log(__func__, "Invalid mach_port");
  }

  free(formatted_message);

  if (response) {
    return response;
  } else {
    return strdup("");
  }
}

void cleanup_global_mach_port() {
  if (g_mach_port != MACH_PORT_NULL) {
    deallocate_mach_port(g_mach_port);
    g_mach_port = MACH_PORT_NULL;
  }
}

void event_server_begin(mach_handler event_handler, char *bootstrap_name) {
  mach_server_begin(&g_mach_server, event_handler, bootstrap_name);
  cleanup_global_mach_port();
}

void free_sketchybar_response(char *response) { free(response); }
