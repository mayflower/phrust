<?php
echo (bool) 0 ? "1" : "0";
echo "|";
echo (bool) 0.0 ? "1" : "0";
echo "|";
echo (bool) "0" ? "1" : "0";
echo "|";
echo (bool) "" ? "1" : "0";
echo "|";
echo (bool) [] ? "1" : "0";
echo "|";
echo (bool) [0] ? "1" : "0";
echo "\n";
