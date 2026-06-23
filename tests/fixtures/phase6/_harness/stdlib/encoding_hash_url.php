<?php
// phase6-diff: id=PHASE6_STDLIB_ENCODING_HASH_URL area=stdlib expect=pass
echo bin2hex("Hi"), "|", hex2bin("4869"), "\n";
echo ord("A"), "|", chr(65), "\n";
echo md5("abc"), "\n";
echo sha1("abc"), "\n";
echo crc32("abc"), "\n";
echo base64_encode("hi"), "|", base64_decode("aGk="), "\n";
echo base64_decode("a!Gk=", false), "|", var_export(base64_decode("a!Gk=", true), true), "\n";
echo htmlspecialchars("<a&>"), "\n";
echo htmlspecialchars_decode("&lt;a&amp;&gt;"), "\n";
echo htmlentities("<a&>"), "\n";
echo urlencode("a b~"), "|", urldecode("a+b%7E"), "\n";
echo rawurlencode("a b~"), "|", rawurldecode("a%20b~"), "\n";
echo http_build_query(["a" => "b", "c" => 1]), "\n";
