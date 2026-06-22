<?php
// phase5-runtime: expect=fail
final class Closed {}
class TryExtend extends Closed {}

echo "unreachable";
