<?php
// stdlib-diff: id=STDLIB_PCRE_BASICS area=json-pcre-date expect=pass
$subject = "route /user/42 and /post/7";
$match = [];
echo preg_match("#/user/(\\d+)#", $subject, $match), "\n";
echo $match[0], "|", $match[1], "\n";
$all = [];
echo preg_match_all("#/(user|post)/(\\d+)#", $subject, $all), "\n";
echo count($all[0]), "|", $all[1][0], "|", $all[2][1], "\n";
