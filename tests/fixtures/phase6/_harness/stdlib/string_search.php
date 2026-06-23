<?php
// phase6-diff: id=PHASE6_STDLIB_STRING_SEARCH area=stdlib expect=pass
echo strlen("abc"), "\n";
echo substr("abcdef", 2, 3), "|", substr("abcdef", -3, 2), "|", substr("abcdef", 2, -1), "\n";
echo strpos("abcabc", "bc"), "|", stripos("AbCd", "bc"), "|", strrpos("abcabc", "a", -1), "\n";
echo str_contains("abc", "") ? "1" : "0";
echo str_starts_with("abc", "ab") ? "1" : "0";
echo str_ends_with("abc", "bc") ? "1" : "0";
echo str_contains("abc", "z") ? "1" : "0";
echo "\n";
echo strcmp("a", "b"), "|", strncmp("abc", "abd", 2), "|", strcasecmp("ABC", "abc"), "|", strncasecmp("ABx", "aby", 2);
echo "\n";
