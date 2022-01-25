use std::os::raw::c_int;

use soem::*;

const EC_TIMEOUTRXM: i32 = 70_000;
const EC_TIMEOUTRET: i32 = 2000;

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
#[repr(packed)]
struct AkdOutputs {
    target_velocity: i32,
    control_word: u16,
}

// Mapped by 0x1b01 - ethercat manual p. 44
#[repr(packed)]
struct AkdInputs {
    position_actual_value: i32,
    status_word: u16,
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

    log::info!("{} slaves found and configured.", c.slaves().len());

    Ok(())
}
