use std::os::raw::c_int;

use soem::*;

const EC_TIMEOUTRXM: i32 = 70_000;

fn akd_setup(slave: u16, context: soem::Context) -> anyhow::Result<()> {
    log::debug!("Setup AKD, PO2SO hook");

    // Clear SM PDO
    context.write_sdo::<u8>(slave, 0x1C12, 00, &0x00, EC_TIMEOUTRXM)?;
    // Clear SM PDO
    context.write_sdo::<u8>(slave, 0x1C13, 00, &0x00, EC_TIMEOUTRXM)?;

    // CSP Fixed PDO
    // context.write_sdo::<u16>(slave, 0x1C12, 01, &0x1701, EC_TIMEOUTRXM)?;
    // Fixed PDO, allows CSP target position
    // context.write_sdo::<u16>(slave, 0x1C12, 01, &0x1724, EC_TIMEOUTRXM)?;
    // Synchronous velocity mode
    context.write_sdo::<u16>(slave, 0x1C12, 01, &0x1702, EC_TIMEOUTRXM)?;

    // One item mapped
    context.write_sdo::<u8>(slave, 0x1C12, 00, &0x01, EC_TIMEOUTRXM)?;
    // Read position from PL.FB instead of FB1.P
    // context.write_sdo::<u16>(slave, 0x1C13, 01, &0x1b24, EC_TIMEOUTRXM)?;
    // Set fixed TXPDO
    context.write_sdo::<u16>(slave, 0x1C13, 01, &0x1B01, EC_TIMEOUTRXM)?;
    // One item mapped
    context.write_sdo::<u8>(slave, 0x1C13, 00, &0x01, EC_TIMEOUTRXM)?;
    // Opmode - Cyclic Synchronous Position
    // context.write_sdo::<u8>(slave, 0x6060, 00, &0x08;, EC_TIMEOUTRXM)?;
    // Opmode - Cyclic Synchronous Velocity
    context.write_sdo::<u8>(slave, 0x6060, 00, &0x09, EC_TIMEOUTRXM)?;

    // Interpolation time period
    context.write_sdo::<u8>(slave, 0x60C2, 01, &0x02, EC_TIMEOUTRXM)?;
    // Interpolation time index
    context.write_sdo::<u8>(slave, 0x60C2, 02, &0xfd, EC_TIMEOUTRXM)?;

    // Scale based on 0x6091 and 0x6092 https://www.kollmorgen.com/en-us/developer-network/position-scaling-akd-drive-ethercat-communication/
    // FBUS.PARAM05
    // context.write_sdo::<u8>(slave, 0x36E9, 00, &0b10000, EC_TIMEOUTRXM)?;
    // FBUS.PARAM05
    context.write_sdo::<u8>(slave, 0x36E9, 00, &0x00, EC_TIMEOUTRXM)?;

    log::info!("Slave configured successfully");

    Ok(())
}

// Mapped by 0x1720 - CSV mode
struct AkdOutputs {
    target_velocity: i32,
    control_word: u16,
}

// Mapped by 0x1b01 - ethercat manual p. 44
struct AkdInputs {
    position_actual_value: i32,
    status_word: u16,
}

fn main() -> anyhow::Result<()> {
    let iface_name = "eth0";

    let mut port: Port = Default::default();
    let mut slaves: [Slave; 1] = Default::default();
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

    let mut io_map: [u8; 4096] = unsafe { std::mem::zeroed() };

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
    .map_err(|err| Err(anyhow::anyhow!("Cannot create context: {}", err)))?;

    c.config_init(false)
        .map_err(|err| Err(anyhow::anyhow!("Cannot configure EtherCat: {}", err)))?;

    log::debug!("Found {} slaves", c.slaves().len());

    let slave = c
        .slaves()
        .get_mut(0)
        .ok_or_else(|| anyhow::anyhow!("No slave!"))?;

    slave.register_po2so(|c, slave| {
        log::debug!("PO2SO hook");
    });

    c.config_map_group(&mut io_map, 0)
        .map_err(|err| Err(anyhow::anyhow!("Cannot configure group map: {}", err)))?;

    c.config_dc()
        .map_err(|err| Err(anyhow::anyhow!("Cannot configure DC: {}", err)))?;

    println!("{} slaves found and configured.", c.slaves().len());

    Ok(())
}
