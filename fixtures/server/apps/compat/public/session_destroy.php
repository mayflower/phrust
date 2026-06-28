<?php
session_start();
echo "id=", session_id(), "\n";
session_destroy();
echo "destroyed=yes\n";
