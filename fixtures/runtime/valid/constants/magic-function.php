<?php
function magic_function_fixture() {
    echo __FUNCTION__, "|", __LINE__, "|", __CLASS__, "|", __METHOD__, "|", __NAMESPACE__, "\n";
}

magic_function_fixture();
