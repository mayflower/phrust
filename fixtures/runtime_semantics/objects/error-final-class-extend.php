<?php
// runtime-semantics: expect=fail
final class Closed {}
class TryExtend extends Closed {}

echo "unreachable";
