--TEST--
wp.request-filesystem: platform surface
--DESCRIPTION--
Generated WordPress request/filesystem surface check for CLI-comparable
functions, extensions, and upload constants.
--FILE--
<?php
foreach ([
    "is_uploaded_file",
    "move_uploaded_file",
    "sys_get_temp_dir",
    "tempnam",
    "tmpfile",
    "chmod",
    "fileperms",
    "fileowner",
    "filegroup",
    "disk_free_space",
    "disk_total_space",
    "opendir",
    "readdir",
    "rewinddir",
    "closedir",
    "scandir",
    "dir",
    "stream_context_create",
    "stream_context_get_options",
    "stream_context_get_default",
    "stream_context_set_default",
    "stream_context_set_option",
    "stream_set_timeout",
] as $name) {
    echo $name, "=", function_exists($name) ? "yes" : "no", "\n";
}
foreach ([
    "UPLOAD_ERR_OK",
    "UPLOAD_ERR_INI_SIZE",
    "UPLOAD_ERR_FORM_SIZE",
    "UPLOAD_ERR_PARTIAL",
    "UPLOAD_ERR_NO_FILE",
    "UPLOAD_ERR_NO_TMP_DIR",
    "UPLOAD_ERR_CANT_WRITE",
    "UPLOAD_ERR_EXTENSION",
] as $name) {
    echo $name, "=", defined($name) ? constant($name) : "missing", "\n";
}
echo "_FILES=", is_array($_FILES) ? "array" : "missing", "\n";
?>
--EXPECT--
is_uploaded_file=yes
move_uploaded_file=yes
sys_get_temp_dir=yes
tempnam=yes
tmpfile=yes
chmod=yes
fileperms=yes
fileowner=yes
filegroup=yes
disk_free_space=yes
disk_total_space=yes
opendir=yes
readdir=yes
rewinddir=yes
closedir=yes
scandir=yes
dir=yes
stream_context_create=yes
stream_context_get_options=yes
stream_context_get_default=yes
stream_context_set_default=yes
stream_context_set_option=yes
stream_set_timeout=yes
UPLOAD_ERR_OK=0
UPLOAD_ERR_INI_SIZE=1
UPLOAD_ERR_FORM_SIZE=2
UPLOAD_ERR_PARTIAL=3
UPLOAD_ERR_NO_FILE=4
UPLOAD_ERR_NO_TMP_DIR=6
UPLOAD_ERR_CANT_WRITE=7
UPLOAD_ERR_EXTENSION=8
_FILES=array
