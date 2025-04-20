REM sudo diskpart /s diskpart-create-vhd.cmd
REM Create a new fixed-size VHD file of 1GB (1024 MB)
create vdisk file="C:\temp\MyDisk.vhd" maximum=1024 type=fixed

REM Select the newly created VHD
select vdisk file="C:\temp\MyDisk.vhd"

REM Attach (mount) the VHD
attach vdisk

REM Create a primary partition in the VHD
create partition primary

REM Format the partition to NTFS using a quick format and assign the label "MyVHD"
format fs=ntfs quick label="MyVHD"

REM Assign a drive letter (for example, E)
assign letter=E