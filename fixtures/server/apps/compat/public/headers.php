<?php
header("X-Compat: alpha");
header("X-Compat: beta");
http_response_code(201);
$headers = headers_list();
echo $headers[0], "\n";
echo http_response_code(), "\n";
echo headers_sent() ? "sent\n" : "not-sent\n";
