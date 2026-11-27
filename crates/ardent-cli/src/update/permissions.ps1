$ErrorActionPreference = 'Stop'
$path = $env:ARDENT_ACL_PATH
$section = [Security.AccessControl.AccessControlSections]::Access

function Set-StagingInheritance($stagingParent, $installedParent) {
    # The staging parent stays private. Children inherit the same rules as at the destination.
    $user = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $acl = [Security.AccessControl.DirectorySecurity]::new()
    $acl.SetAccessRuleProtection($true, $false)
    $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($user, 'FullControl', 'Allow'))
    $source = [Security.AccessControl.RawSecurityDescriptor]::new(
        $installedParent.GetAccessControl($section).GetSecurityDescriptorBinaryForm(), 0)
    $private = [Security.AccessControl.RawSecurityDescriptor]::new($acl.GetSecurityDescriptorBinaryForm(), 0)
    $inherit = [Security.AccessControl.AceFlags]'ContainerInherit,ObjectInherit'
    $position = 0
    foreach ($rule in $source.DiscretionaryAcl) {
        if ($rule.AceFlags -band $inherit) {
            $copy = $rule.Copy()
            $copy.AceFlags = ($copy.AceFlags -band (-bnot [Security.AccessControl.AceFlags]::Inherited)) -bor
                [Security.AccessControl.AceFlags]::InheritOnly
            $private.DiscretionaryAcl.InsertAce($position, $copy)
            $position++
        }
    }
    $bytes = [byte[]]::new($private.BinaryLength)
    $private.GetBinaryForm($bytes, 0)
    $acl.SetSecurityDescriptorBinaryForm($bytes, $section)
    $stagingParent.SetAccessControl($acl)
}

function Get-Dacl($acl) {
    # Auto-inheritance bookkeeping flags can differ after a move; ACEs and protection determine access.
    $acl.GetSecurityDescriptorSddlForm($section) -creplace '^D:(?:AI|AR|P)*', "$($acl.AreAccessRulesProtected):"
}

function Sync-PackageAccess($source, $target, [switch]$Verify) {
    if (($source.Attributes -bor $target.Attributes) -band [IO.FileAttributes]::ReparsePoint) {
        throw "Cannot copy access through a reparse point at $($target.FullName)"
    }
    # Set child-specific restrictions before any parent grants can propagate to them.
    if ($target -is [IO.DirectoryInfo]) {
        $old = [Collections.Generic.Dictionary[string,IO.FileSystemInfo]]::new([StringComparer]::Ordinal)
        foreach ($entry in $source.GetFileSystemInfos()) {
            $old.Add($entry.Name, $entry)
        }
        foreach ($child in $target.GetFileSystemInfos()) {
            $previous = $null
            if ($old.TryGetValue($child.Name, [ref]$previous) -and $previous.GetType() -eq $child.GetType() -and
                !($previous.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
                Sync-PackageAccess $previous $child -Verify:$Verify
            }
        }
    }
    $wanted = $source.GetAccessControl($section)
    $current = $target.GetAccessControl($section)
    if ((Get-Dacl $current) -cne (Get-Dacl $wanted)) {
        if ($Verify) { throw "Could not preserve access to $($source.FullName)" }
        $current.SetSecurityDescriptorBinaryForm($wanted.GetSecurityDescriptorBinaryForm(), $section)
        $target.SetAccessControl($current)
    }
}

if ($env:ARDENT_ACL_ACTION -in 'prepare', 'repair') {
    $source = [IO.DirectoryInfo]::new($env:ARDENT_PREVIOUS_ACL)
    $target = [IO.DirectoryInfo]::new($path)
    Sync-PackageAccess $source $target
    if ($env:ARDENT_ACL_ACTION -eq 'prepare') {
        Set-StagingInheritance $target.Parent $source.Parent
    }
    Sync-PackageAccess $source $target -Verify
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
