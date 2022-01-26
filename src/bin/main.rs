use std::{
    mem::size_of,
    os::raw::c_int,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use soem::*;

const EC_TIMEOUTRXM: i32 = 70_000;
const EC_TIMEOUTRET: i32 = 2000;
const EC_TIMEOUTSTATE: i32 = 2000000;

trait NilError<C> {
    fn nil_err(self, context: C) -> Result<(), ()>;
}

impl<'a, C> NilError<C> for Result<(), ErrorIterator<'a>>
where
    C: std::fmt::Display,
{
    fn nil_err(self, context: C) -> Result<(), ()> {
        self.map_err(|e| {
            let msg = e.collect::<Vec<_>>();

            log::error!("Write SDO error ({}): {:?}", context, msg);

            ()
        })
    }
}

fn akd_setup(ctx: &mut soem::Context, slave: u16) -> Result<(), ()> {
    log::debug!("Setup AKD, PO2SO hook");

    // Clear SM PDO
    ctx.write_sdo::<u8>(slave, 0x1C12, 00, &0x00, EC_TIMEOUTRXM)
        .nil_err("0x1C12:00")?;
    // Clear SM PDO
    ctx.write_sdo::<u8>(slave, 0x1C13, 00, &0x00, EC_TIMEOUTRXM)
        .nil_err("0x1C13:00")?;

    // CSP Fixed PDO
    // ctx.write_sdo::<u16>(slave, 0x1C12, 01, &0x1701, EC_TIMEOUTRXM).nil_err()?;
    // Fixed PDO, allows CSP target position
    // ctx.write_sdo::<u16>(slave, 0x1C12, 01, &0x1724, EC_TIMEOUTRXM).nil_err()?;
    // Synchronous velocity mode
    ctx.write_sdo::<u16>(slave, 0x1C12, 01, &0x1702, EC_TIMEOUTRXM)
        .nil_err("0x1C12:01")?;

    // One item mapped
    ctx.write_sdo::<u8>(slave, 0x1C12, 00, &0x01, EC_TIMEOUTRXM)
        .nil_err("0x1C12:00")?;
    // Read position from PL.FB instead of FB1.P
    // ctx.write_sdo::<u16>(slave, 0x1C13, 01, &0x1b24, EC_TIMEOUTRXM).nil_err()?;
    // Set fixed TXPDO
    ctx.write_sdo::<u16>(slave, 0x1C13, 01, &0x1B01, EC_TIMEOUTRXM)
        .nil_err("0x1C13:01")?;
    // One item mapped
    ctx.write_sdo::<u8>(slave, 0x1C13, 00, &0x01, EC_TIMEOUTRXM)
        .nil_err("0x1C13:00")?;
    // Opmode - Cyclic Synchronous Position
    // ctx.write_sdo::<u8>(slave, 0x6060, 00, &0x08, EC_TIMEOUTRXM).nil_err()?;
    // Opmode - Cyclic Synchronous Velocity
    ctx.write_sdo::<u8>(slave, 0x6060, 00, &0x09, EC_TIMEOUTRXM)
        .nil_err("0x6060:00")?;

    // Interpolation time period
    ctx.write_sdo::<u8>(slave, 0x60C2, 01, &0x02, EC_TIMEOUTRXM)
        .nil_err("0x60C2:01")?;
    // Interpolation time index
    ctx.write_sdo::<u8>(slave, 0x60C2, 02, &0xfd, EC_TIMEOUTRXM)
        .nil_err("0x60C2:02")?;

    // Scale based on 0x6091 and 0x6092 https://www.kollmorgen.com/en-us/developer-network/position-scaling-akd-drive-ethercat-communication/
    // FBUS.PARAM05
    // ctx.write_sdo::<u8>(slave, 0x36E9, 00, &0b10000, EC_TIMEOUTRXM).nil_err()?;
    // FBUS.PARAM05
    ctx.write_sdo::<u32>(slave, 0x36E9, 00, &0x00, EC_TIMEOUTRXM)
        .nil_err("0x36E9:00")?;

    log::info!("Slave configured successfully");

    Ok(())
}

// Mapped by 0x1720 - CSV mode
#[derive(Debug, Copy, Clone)]
// C packing retains ordering and does not add padding
#[repr(C, packed)]
struct AkdOutputs {
    target_velocity: i32,
    control_word: u16,
}

// Mapped by 0x1b01 - ethercat manual p. 44
#[derive(Debug, Copy, Clone)]
// C packing retains ordering and does not add padding
#[repr(C, packed)]
struct AkdInputs {
    position_actual_value: i32,
    status_word: u16,
}

// TODO: Realtime thread sleep?
fn sleep_5000() {
    std::thread::sleep(Duration::from_micros(5000));
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let iface_name = "eth0";

    let mut port: Port = Default::default();
    let mut slaves: [Slave; 8] = Default::default();
    let mut slavecount: c_int = Default::default();
    let mut groups: [Group; 2] = Default::default();
    let mut esibuf: ESIBuf = Default::default();
    let mut esimap: ESIMap = Default::default();
    let mut elist: ERing = Default::default();
    let mut idxstack: IdxStack = Default::default();
    let mut ecaterror: Boolean = Default::default();
    let mut dc_time: i64 = Default::default();
    let mut sm_commtype: SMCommType = Default::default();
    let mut pdo_assign: PDOAssign = Default::default();
    let mut pdo_desc: PDODesc = Default::default();
    let mut eep_sm: EEPROMSM = Default::default();
    let mut eep_fmmu: EEPROMFMMU = Default::default();

    log::info!("EtherCAT starting on {}", iface_name);

    let mut io_map: [u8; 4096] = [0u8; 4096];

    let mut c = Context::new(
        iface_name,
        &mut port,
        &mut slaves,
        &mut slavecount,
        &mut groups,
        &mut esibuf,
        &mut esimap,
        &mut elist,
        &mut idxstack,
        &mut ecaterror,
        &mut dc_time,
        &mut sm_commtype,
        &mut pdo_assign,
        &mut pdo_desc,
        &mut eep_sm,
        &mut eep_fmmu,
    )
    .map_err(|err| anyhow::anyhow!("Cannot create context: {}", err))?;

    c.config_init(false)
        .map_err(|err| anyhow::anyhow!("Cannot configure EtherCat: {}", err))?;

    log::debug!("Found {} slaves", c.slaves().len());

    {
        let slave = c
            .slaves()
            .get_mut(0)
            .ok_or_else(|| anyhow::anyhow!("No slave!"))?;

        log::debug!(
            "Got slave 0: {:#0x} {:#0x}",
            slave.eep_manufacturer(),
            slave.eep_id()
        );

        // Kollmorgen AKD
        assert_eq!(slave.eep_manufacturer(), 0x6a);
        assert_eq!(slave.eep_id(), 0x414b44);

        slave.register_po2so(akd_setup);
    }

    log::debug!("Registered PO2SO hook");

    c.config_map_group(&mut io_map, 0)
        .map_err(|err| anyhow::anyhow!("Cannot configure group map: {}", err))?;

    log::debug!("Config map done");

    c.config_dc()
        .map_err(|err| anyhow::anyhow!("Cannot configure DC: {}", err))?;

    log::debug!("DC done");

    for _ in 0..15000 {
        c.send_processdata();
        c.receive_processdata(EC_TIMEOUTRET);
    }

    log::info!("{} slaves mapped, state to SAFE_OP.", c.slaves().len());

    c.check_state(0, EtherCatState::SafeOp, EC_TIMEOUTSTATE * 4);

    // Cast inputs/outputs to a nice struct
    // TODO

    c.set_state(EtherCatState::Op, 0);

    // Send process data once to make outputs happy
    c.send_processdata();
    c.receive_processdata(EC_TIMEOUTRET);

    // let expected_wkc = c.groups()[0].expected_wkc();

    c.write_state(0)?;

    // Wait for slave to enter op state
    for _ in 0..200 {
        match c.check_state(0, EtherCatState::Op, 50000) {
            EtherCatState::Op => break,
            _ => {
                c.send_processdata();
                c.receive_processdata(EC_TIMEOUTRET);
            }
        }
    }

    log::trace!("Wait loop done");

    // Exit if slave didn't reach operational state
    if c.read_state() != EtherCatState::Op {
        let e = anyhow::anyhow!("Cannot reach {} state for the slaves", EtherCatState::Op);

        log::error!("{}", e);

        let slave = c
            .slaves()
            .get_mut(0)
            .ok_or_else(|| anyhow::anyhow!("No slave!"))?;

        match slave.state() {
            EtherCatState::Op => (),
            state => {
                log::error!("Slave 0 ({}) in state {}", slave.name(), state);
            }
        }

        return Err(e);
    }

    log::info!("Slaves reached OP state");

    // Slaves reached op state. Need to spawn a thread here to check/update state
    // TODO: Spawn thread

    {
        let slave = c
            .slaves()
            .get_mut(0)
            .ok_or_else(|| anyhow::anyhow!("No slave!"))?;

        // TODO: Move to an `{inputs|outputs}_cast` method on `Slave`. Maybe I can use Box here?
        let in_ptr = {
            let buf = slave.inputs();

            assert_eq!(buf.len(), size_of::<AkdInputs>(), "inputs not castable");

            let in_ptr: &AkdInputs = unsafe { std::mem::transmute(buf.as_ptr()) };

            in_ptr
        };

        let out_ptr = {
            let buf = slave.outputs();

            assert_eq!(buf.len(), size_of::<AkdOutputs>(), "outputs not castable");

            let out_ptr: &mut AkdOutputs = unsafe { std::mem::transmute(buf.as_mut_ptr()) };

            out_ptr
        };

        log::debug!("Inputs {:?}", in_ptr);
        log::debug!("Outputs {:?}", out_ptr);

        // If we've faulted, clear faults by setting clear fault flag high
        if (in_ptr.status_word & 0b1000) > 0x0 {
            out_ptr.control_word = 0x80; //clear errors, rising edge

            // Fault flag is bit 4, wait for clear
            loop {
                log::debug!("Wait for 6040 fault cleared, got {:#04x}", {
                    (*in_ptr).status_word
                });

                c.send_processdata();
                c.receive_processdata(EC_TIMEOUTRET);

                sleep_5000();

                if (in_ptr.status_word & 0b1000) > 0 {
                    break;
                }
            }
        }

        // Shutdown
        out_ptr.control_word = 0x6;

        // ready to switch on, wait for it to be set
        loop {
            log::debug!("Wait for 6040 fault cleared again, got {:#04x}", {
                (*in_ptr).status_word
            });

            c.send_processdata();
            c.receive_processdata(EC_TIMEOUTRET);

            sleep_5000();

            if (in_ptr.status_word & 0b1) == 0 {
                break;
            }
        }

        // Switch on - this disengages the brake and "primes" the servo, but won't accept motion
        // commands yet.
        out_ptr.control_word = 0x7;

        // switched on, wait for bit to be set
        loop {
            log::debug!("Wait for 6040 switch on, got {:#04x}", {
                (*in_ptr).status_word
            });

            c.send_processdata();
            c.receive_processdata(EC_TIMEOUTRET);

            sleep_5000();

            if (in_ptr.status_word & 0b10) == 0 {
                break;
            }
        }

        // Prevent motor from jumping on startup
        out_ptr.target_velocity = 0;

        // Enable operation - starts accepting motion comments
        out_ptr.control_word = 0xf;

        // operation enable, wait for bit to be set
        loop {
            log::debug!("Wait for 6040 switch on, got {:#04x}", {
                (*in_ptr).status_word
            });

            c.send_processdata();
            c.receive_processdata(EC_TIMEOUTRET);

            sleep_5000();

            if (in_ptr.status_word & 0b100) == 0 {
                break;
            }
        }

        log::info!("AKD state transitioned to Enable Operation\n");
    }

    let slave = c
        .slaves()
        .get_mut(0)
        .ok_or_else(|| anyhow::anyhow!("No slave!"))?;

    // TODO: Move to an `{inputs|outputs}_cast` method on `Slave`. Maybe I can use Box here?
    let in_ptr = {
        let buf = slave.inputs();

        assert_eq!(buf.len(), size_of::<AkdInputs>(), "inputs not castable");

        let in_ptr: &AkdInputs = unsafe { std::mem::transmute(buf.as_ptr()) };

        in_ptr
    };

    let out_ptr = {
        let buf = slave.outputs();

        assert_eq!(buf.len(), size_of::<AkdOutputs>(), "outputs not castable");

        let out_ptr: &mut AkdOutputs = unsafe { std::mem::transmute(buf.as_mut_ptr()) };

        out_ptr
    };

    let mut pos = 0;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        c.send_processdata();
        let wkc = c.receive_processdata(EC_TIMEOUTRET);

        // TODO: Handle working counter

        if pos < 1_000_000 {
            pos += 1000;
        }

        out_ptr.target_velocity = pos;

        log::info!(
            "WKC {} T: {}, pos {}, status {:#04x}",
            wkc,
            c.dc_time(),
            { (*in_ptr).position_actual_value },
            { (*in_ptr).status_word },
        );

        sleep_5000()
    }

    c.set_state(EtherCatState::Init, 0);
    c.write_state(0);

    Ok(())
}
