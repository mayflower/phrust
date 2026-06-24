<?php
echo "before|", eval('echo "inner|"; return 7;'), "|after\n";
