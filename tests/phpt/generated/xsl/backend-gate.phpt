--TEST--
xsl: XSLTProcessor backend gate
--DESCRIPTION--
Focused XML-family coverage for the XSL class owner and missing libxslt backend gate.
--SKIPIF--
<?php
if (basename(PHP_BINARY) !== "phrust-php") {
    die("skip phrust-only XSL facade fixture");
}
?>
--FILE--
<?php
echo extension_loaded('xsl') ? "loaded\n" : "missing\n";
echo class_exists('XSLTProcessor', false) ? "XSLTProcessor class\n" : "XSLTProcessor missing\n";
echo method_exists('XSLTProcessor', 'hasExsltSupport') ? "hasExsltSupport method\n" : "hasExsltSupport missing\n";
$class = new ReflectionClass('XSLTProcessor');
echo $class->getName(), "|", $class->getExtensionName(), "|", ($class->isInternal() ? "internal" : "user"), "\n";
new XSLTProcessor();
?>
--EXPECTF--
loaded
XSLTProcessor class
hasExsltSupport method
XSLTProcessor|xsl|internal
%s: runtime-diagnostic: %s"E_PHP_VM_UNSUPPORTED_XSL"%slibxslt backend capability gate%s
%s: runtime_error: E_PHP_VM_UNSUPPORTED_XSL: class %s requires a libxslt backend capability gate
