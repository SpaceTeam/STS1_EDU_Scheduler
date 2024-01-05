# STS1 EDU Scheduler

This software is the main executable run on the EDU. Its task is to receive commands from the COBC and execute them accordingly.

## Installing

Under githubs action page you can always find the binary crosscompiled for the raspi. Extract the provided archive into `/opt/scheduler` and simply execute `./STS1_EDU_Scheduler` manually, or symlink (`ln -s /opt/scheduler/scheduler.service /etc/systemd/system/scheduler.service`) the service file and enable it (`systemctl daemon-reload && systemctl enable scheduler`) to autostart.

## Further information

Compilation/Testing/Setup Help: [Wiki](https://github.com/SpaceTeam/STS1_EDU_Scheduler/wiki)

Design Document: [PDD](https://www.overleaf.com/project/628ce894934b28e99889f531)
