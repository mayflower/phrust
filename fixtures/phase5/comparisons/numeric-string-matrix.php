<?php
echo (42 == " 42") ? "1" : "0";
echo "|";
echo (42 == "42abc") ? "1" : "0";
echo "|";
echo (0 == "foo") ? "1" : "0";
echo "|";
echo ("0e123" == "0") ? "1" : "0";
echo "|";
echo ("42abc" == "42") ? "1" : "0";
echo "\n";
