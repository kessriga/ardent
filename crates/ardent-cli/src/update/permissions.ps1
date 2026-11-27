$ErrorActionPreference = 'Stop'
$path = $env:ARDENT_ACL_PATH
if ($env:ARDENT_ACL_ACTION -eq 'restore') {
    $section = [Security.AccessControl.AccessControlSections]::Access
    $original = [IO.Directory]::GetAccessControl($env:ARDENT_PREVIOUS_ACL, $section)
    $acl = New-Object Security.AccessControl.DirectorySecurity
    $acl.SetSecurityDescriptorBinaryForm($original.GetSecurityDescriptorBinaryForm(), $section)
    [IO.Directory]::SetAccessControl($path, $acl)
    return
}

$user = [Security.Principal.WindowsIdentity]::GetCurrent().User
$acl = New-Object Security.AccessControl.DirectorySecurity
$acl.SetOwner($user)
$acl.SetAccessRuleProtection($true, $false)
$rule = [Security.AccessControl.FileSystemAccessRule]::new(
    $user, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
$acl.AddAccessRule($rule)
if ($env:ARDENT_ACL_ACTION -eq 'create') {
    if ([IO.Directory]::Exists($path)) { throw 'Staging directory already exists' }
    [void][IO.Directory]::CreateDirectory($path, $acl)
    return
}
$dir = [IO.DirectoryInfo]::new($path)
$actual = $dir.GetAccessControl()
$rules = @($actual.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier]))
if (($dir.Attributes -band [IO.FileAttributes]::ReparsePoint) -or
    !$actual.AreAccessRulesProtected -or $actual.GetOwner([Security.Principal.SecurityIdentifier]) -ne $user -or
    $rules.Count -ne 1 -or $rules[0].IdentityReference -ne $user -or
    $rules[0].InheritanceFlags -ne $rule.InheritanceFlags -or
    $rules[0].AccessControlType -ne $rule.AccessControlType -or
    $rules[0].FileSystemRights -ne $rule.FileSystemRights) {
    throw 'The filesystem could not create a private staging directory'
}
