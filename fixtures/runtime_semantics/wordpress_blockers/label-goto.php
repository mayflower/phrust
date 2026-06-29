<?php
// runtime-semantics: category=wordpress_blockers expect=pass
$i = 0;
again:
$i++;
if ($i < 3) {
    goto again;
}
goto done;
echo "skip\n";
done:
echo $i, "\n";
