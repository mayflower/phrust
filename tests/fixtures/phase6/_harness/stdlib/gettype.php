<?php
// phase6-diff: id=PHASE6_STDLIB_GETTYPE area=stdlib expect=pass
echo gettype(null), "|", gettype(7), "|", gettype("x"), "\n";
