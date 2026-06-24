<?php
// runtime-fixture: kind=valid expected_stdout="1|1|1|1|-1\n"
echo 1 == "1", "|", 1 === 1, "|", 1 !== "1", "|", 2 >= 2, "|", 2 <=> 3, "\n";
