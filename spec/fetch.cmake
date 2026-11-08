# Downloads the AARCHMRS release pinned in REVISION, verifies its SHA-256 and unpacks
# it.
#
#   cmake [-DDEST=<dir>] -P fetch.cmake
#
# DEST defaults to $AARCHMRS_DATA_DIR/<release> when AARCHMRS_DATA_DIR is set, else
# to `data/<release>` next to this script (ignored by git). The spec files are
# checked against the `file` hashes after unpacking. Run decoderant-a64-gen with
# `--spec <DEST>` afterwards.
cmake_minimum_required(VERSION 3.20)

file(STRINGS "${CMAKE_CURRENT_LIST_DIR}/REVISION" lines REGEX "^[a-z]+ ")
set(files "")
foreach(line IN LISTS lines)
  if(line MATCHES "^release ([^ ]+)$")
    set(release "${CMAKE_MATCH_1}")
  elseif(line MATCHES "^tarball ([0-9a-f]+) (https://[^ ]+)$")
    set(tar_hash "${CMAKE_MATCH_1}")
    set(tar_url "${CMAKE_MATCH_2}")
  elseif(line MATCHES "^file ([^ ]+) ([0-9a-f]+)$")
    list(APPEND files "${CMAKE_MATCH_1}=${CMAKE_MATCH_2}")
  else()
    message(FATAL_ERROR "malformed REVISION line: ${line}")
  endif()
endforeach()
if(NOT release OR NOT tar_hash)
  message(FATAL_ERROR "REVISION needs a release and a tarball line")
endif()

if(NOT DEFINED DEST)
  if(DEFINED ENV{AARCHMRS_DATA_DIR})
    set(DEST "$ENV{AARCHMRS_DATA_DIR}/${release}")
  else()
    set(DEST "${CMAKE_CURRENT_LIST_DIR}/data/${release}")
  endif()
endif()
file(MAKE_DIRECTORY "${DEST}")

set(tarball "${DEST}/${release}.tar.gz")
if(EXISTS "${tarball}")
  file(SHA256 "${tarball}" have)
  if(NOT have STREQUAL tar_hash)
    file(REMOVE "${tarball}")
  endif()
endif()
if(NOT EXISTS "${tarball}")
  message(STATUS "fetching ${release}")
  file(DOWNLOAD "${tar_url}" "${tarball}" EXPECTED_HASH SHA256=${tar_hash} TLS_VERIFY ON
       STATUS st)
  list(GET st 0 code)
  if(NOT code EQUAL 0)
    file(REMOVE "${tarball}")
    message(FATAL_ERROR "download of ${tar_url} failed: ${st}")
  endif()
endif()

file(ARCHIVE_EXTRACT INPUT "${tarball}" DESTINATION "${DEST}")

foreach(entry IN LISTS files)
  string(REPLACE "=" ";" parts "${entry}")
  list(GET parts 0 path)
  list(GET parts 1 hash)
  file(SHA256 "${DEST}/${path}" have)
  if(NOT have STREQUAL hash)
    message(FATAL_ERROR "${path}: SHA-256 ${have}, REVISION pins ${hash}")
  endif()
endforeach()
message(STATUS "AARCHMRS ${release} ready in ${DEST}")
