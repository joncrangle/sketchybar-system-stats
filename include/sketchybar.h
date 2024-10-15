#pragma once

#include <bootstrap.h>
#include <mach/mach.h>
#include <mach/message.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>

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

char *mach_send_message(mach_port_t port, const char *message, uint32_t len) {
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
    mach_port_mod_refs(task, response_port, MACH_PORT_RIGHT_RECEIVE, -1);
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
    }
  }

  mach_msg_destroy(&buffer.message.header);
  mach_port_mod_refs(task, response_port, MACH_PORT_RIGHT_RECEIVE, -1);
  mach_port_deallocate(task, response_port);

  return result;
}

char *sketchybar(const char *message, const char *bar_name) {
  if (!message || !bar_name) {
    return strdup("");
  }

  uint32_t message_length = strlen(message) + 1;
  char *formatted_message = (char *)malloc(message_length + 1);
  if (!formatted_message) {
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

  pthread_mutex_lock(&g_port_mutex);
  if (g_mach_port == MACH_PORT_NULL) {
    g_mach_port = mach_get_bs_port((char *)bar_name);
  }

  char *response = mach_send_message(g_mach_port, formatted_message, caret + 1);
  pthread_mutex_unlock(&g_port_mutex);

  free(formatted_message);

  return response ? response : strdup("");
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
