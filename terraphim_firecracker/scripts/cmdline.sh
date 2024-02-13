#!/usr/bin/env bash
############################################################################################
# Linux Kernel Commandline Parsing Shellscript Functions
# 
# These functions have been written in a way that they should be compatible
# with almost any "standard" UNIX/Linux shell, including: sh, bash, ash, dash, zsh
#
# They are designed to be sourced from (or copy pasted into) another shellscript,
# and used to parse / query the Linux kernel commandline ( /proc/cmdline ), or strings
# which use a similar key=value format.
#
############################################################################################
#
# Function Index:
# 
#    - get_cmd [key]   - Parses /proc/cmdline - looking for KEY=VALUE, then outputs VALUE 
#                        to stdout after extracting it with regex. Supports quoted value's 
#                        containing spaces. 
#
#    - has_cmd [key]   - Similar to get_cmd except it doesn't output anything. It returns 
#                        boolean 0 or 1 return codes depending on whether or not 'key=?' 
#                        is present in /proc/cmdline
#
#    - has_cmd_word [key]    - Similar to 'has_cmd', except only detects "words", 
#                              i.e. standalone strings inside of /proc/cmdline without
#                              an '=VALUE' - e.g. 'ro', 'quiet', 'safemode'
#
#    - extractpr        - This is the lower level function that powers get_cmd/has_cmd - 
#                         it handles running sed regex against piped stdin, and running 
#                         a second-layer regex to handle quoted values if it detects 
#                         the first search starts with a "
#
#############################################################################################
#                                                                                           #
#                     (C) Copyright 2020 by Someguy123 / Privex Inc.                        #
#                                                                                           #
#                 Released under the X11/MIT License (bottom of the file)                   #
#                                                                                           #
#     https://github.com/Someguy123    github.com/Privex    https://www.privex.io           #
#                                                                                           #
#############################################################################################

################
# Extract a parameter-like value from a string piped into stdin
#
# $ echo "hello world=\"an example\" test=123" | extractpr world
# an example
# $ echo "hello world=\"an example\" test=123" | extractpr test
# 123
#
extractpr() {
  data_in="$(cat)"
  tag="$1"
  # Standard regex matcher to extract the value for a given tag
  m_matcher="([a-zA-Z0-9/\\@#\$%^&\!*\(\)'\"=:,._-]+)"
  # Regex matcher for values that are quoted
  q_matcher="\"([a-zA-Z0-9/\\@#\$%^&\!*\(\)',=: ._-]+)\""
  [ $# -gt 1 ] && m_matcher="$2" && q_matcher="$2"
  k_res="$(printf '%s' "$data_in" | sed -rn "s/.* ?${tag}=${m_matcher} ?(.*)+?/\1/p")"
  if echo "$k_res" | grep -Eq '^"'; then
    k_res="$(printf "%s\n" "$data_in" | sed -rn "s/.* ?${tag}=${q_matcher} ?(.*)+?/\1/p")"
  fi
  printf "%s\n" "$k_res"
}

################
# get_cmd [name] (matcher)
# Extract the value of a given key=value pair in /proc/cmdline
# It DOES NOT match single words inside of the kernel cmdline - for words, use has_cmd_word.
# e.g.
#
#   $ cat /proc/cmdline
#   BOOT_IMAGE=/boot/vmlinuz-4.15.0-122-generic root=UUID=abcd1-2487-4cc1-bc38-11fd08ba1a0a 
#   URLS="http://example.com http://lorem.ipsum" ro safe quiet
#   $ get_cmd BOOT_IMAGE
#   /boot/vmlinuz-4.15.0-122-generic
#   $ get_cmd root
#   UUID=abcd1-2487-4cc1-bc38-11fd08ba1a0a
#   $ get_cmd URLS
#   http://example.com http://lorem.ipsum
#
get_cmd() {
  res=$(cat /proc/cmdline | extractpr "$@")
  if [ -z "$res" ]; then
    return 1
  else
    printf "%s\n" "$res"
  fi
}


################
# has_cmd [name] (matcher)
# Returns 0 if /proc/cmdline contains a given key+value entry.
# It DOES NOT match single words inside of the kernel cmdline - for words, use has_cmd_word.
# e.g.
#
#   $ cat /proc/cmdline
#   BOOT_IMAGE=/boot/vmlinuz-4.15.0-122-generic root=UUID=abcd1-2487-4cc1-bc38-11fd08ba1a0a ro safe quiet
#   $ has_cmd BOOT_IMAGE && echo yes || echo no
#   yes
#   $ has_cmd root && echo yes || echo no
#   yes
#   $ has_cmd quiet && echo yes || echo no
#   no
has_cmd() {
  get_cmd "$@" > /dev/null;
}

################
# has_cmd_word [name] [name2] [name3] ...
# Returns 0 (true) if AT LEAST one of the passed "words" exist in the cmdline
# If none of the passed words are found, will return 1 (false)
# Acts as an OR query for names inside of cmdline
# e.g.
#
#   $ cat /proc/cmdline
#   BOOT_IMAGE=/boot/somekernel root=/dev/sda ro enablex usey world="lorem ipsum"
#   $ has_cmd_word ro && echo yes || echo no
#   yes
#   $ has_cmd_word roo && echo yes || echo no
#   no
#   $ has_cmd_word root && echo yes || echo no
#   yes
#   $ has_cmd_word enablex && echo yes || echo no
#   yes
#
has_cmd_word() {
  for k in "$@"; do
    if cat /proc/cmdline | grep -Eq " ${k}(=.*)? |^${k}(=.*)? | ${k}(=.*)?\$"; then
      return 0
    fi
  done
  return 1
}



#############################################################################################
#                                                                                           #
#                     (C) Copyright 2020 by Someguy123 / Privex Inc.                        #
#                                                                                           #
#                 Released under the X11/MIT License (bottom of the file)                   #
#                                                                                           #
#     https://github.com/Someguy123    github.com/Privex    https://www.privex.io           #
#                                                                                           #
#############################################################################################
#
#    Permission is hereby granted, free of charge, to any person obtaining a copy of 
#    this software and associated documentation files (the "Software"), to deal in 
#    the Software without restriction, including without limitation the rights to use, 
#    copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the 
#    Software, and to permit persons to whom the Software is furnished to do so, 
#    subject to the following conditions:
#
#    The above copyright notice and this permission notice shall be included in all 
#    copies or substantial portions of the Software.
#
#    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, 
#    INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A 
#    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT 
#    HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION 
#    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE 
#    SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
#
#############################################################################################
