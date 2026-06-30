<?php
// runtime-semantics: expect=fail
eval('class EvalRedeclaredSymbolClass {}');
eval('class EvalRedeclaredSymbolClass {}');
