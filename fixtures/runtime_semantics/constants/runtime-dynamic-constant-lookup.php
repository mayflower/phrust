<?php
define("DYNAMIC_LOOKUP_CONST", "dynamic");
echo defined("DYNAMIC_LOOKUP_CONST") ? "yes|" : "no|";
echo constant("DYNAMIC_LOOKUP_CONST"), "|", DYNAMIC_LOOKUP_CONST, "\n";
