<?php
// stdlib-diff: id=STDLIB_HASH_RANDOM area=stdlib expect=pass
echo hash("sha1", "abc"), "\n";
echo bin2hex(hash("md5", "abc", true)), "\n";
echo hash("crc32b", "abc"), "\n";
echo hash_hmac("sha1", "data", "key"), "\n";
$bytes = random_bytes(8);
echo strlen($bytes), "|", strlen(bin2hex($bytes)), "\n";
$value = random_int(2, 4);
echo ($value >= 2 && $value <= 4) ? "range\n" : "bad\n";
