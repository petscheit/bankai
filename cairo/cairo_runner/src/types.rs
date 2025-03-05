pub use garaga_zero_hints::types::CairoType;
use cairo_vm::{types::relocatable::Relocatable, vm::{errors::memory_errors::MemoryError, vm_core::VirtualMachine}, Felt252};
use num_bigint::BigUint;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(try_from = "String")]
pub struct Uint256(pub BigUint);

impl TryFrom<String> for Uint256 {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let hex_str = value.strip_prefix("0x").unwrap_or(&value);
        BigUint::parse_bytes(hex_str.as_bytes(), 16)
            .map(Uint256)
            .ok_or_else(|| format!("Invalid hex string: {}", value))
    }
}

impl Uint256 {
    pub fn to_limbs(&self) -> [Felt252; 2] {
        const LIMB_SIZE: u32 = 128;
        let limb_mask = (BigUint::from(1u128) << LIMB_SIZE) - BigUint::from(1u128);
        
        let lower_limb = &self.0 & &limb_mask;
        let upper_limb = &self.0 >> LIMB_SIZE;
        
        [
            Felt252::from_bytes_be_slice(&lower_limb.to_bytes_be()),
            Felt252::from_bytes_be_slice(&upper_limb.to_bytes_be())
        ]
    }
}

impl CairoType for Uint256 {
    fn from_memory(vm: &VirtualMachine, address: Relocatable) -> Result<Self, MemoryError> {
        let d0 = BigUint::from_bytes_be(&vm.get_integer((address + 0)?)?.to_bytes_be());
        let d1 = BigUint::from_bytes_be(&vm.get_integer((address + 1)?)?.to_bytes_be());
        let bigint = d1 << 128 | d0;
        Ok(Self(bigint))
    }

    fn to_memory(&self, vm: &mut VirtualMachine, address: Relocatable) -> Result<Relocatable, MemoryError> {
        let limbs = self.to_limbs();
        vm.insert_value((address + 0)?, &limbs[0])?;
        vm.insert_value((address + 1)?, &limbs[1])?;
        Ok((address + 2)?)
    }

    fn n_fields(_vm: &VirtualMachine, _address: Relocatable) -> Result<usize, MemoryError> {
        Ok(2)
    }
}


#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
pub struct Uint256Bits32(pub BigUint);

impl TryFrom<String> for Uint256Bits32 {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let hex_str = value.strip_prefix("0x").unwrap_or(&value);
        BigUint::parse_bytes(hex_str.as_bytes(), 16)
            .map(Uint256Bits32)
            .ok_or_else(|| format!("Invalid hex string: {}", value))
    }
}

impl Uint256Bits32 {
    pub fn to_limbs(&self) -> [Felt252; 8] {
        const LIMB_SIZE: u32 = 32;
        let limb_mask = (BigUint::from(1u64) << LIMB_SIZE) - BigUint::from(1u64);
        
        let limbs = (0..8).map(|i| {
            let shift = (7 - i) * LIMB_SIZE;
            let limb = (&self.0 >> shift) & &limb_mask;
            Felt252::from_bytes_be_slice(&limb.to_bytes_be())
        }).collect::<Vec<_>>();
        
        limbs.try_into().unwrap()
    }
}

impl CairoType for Uint256Bits32 {
    fn from_memory(vm: &VirtualMachine, address: Relocatable) -> Result<Self, MemoryError> {
        let mut bigint = BigUint::from(0u32);
        
        for i in (0..8).rev() {
            let value = BigUint::from_bytes_be(&vm.get_integer((address + i)?)?.to_bytes_be());
            bigint = (bigint << 32) | value;
        }
        
        Ok(Self(bigint))
    }

    fn to_memory(&self, vm: &mut VirtualMachine, address: Relocatable) -> Result<Relocatable, MemoryError> {
        let limbs = self.to_limbs();
        
        for i in 0..8 {
            vm.insert_value((address + i)?, &limbs[i])?;
        }
        
        Ok((address + 8)?)
    }

    fn n_fields(_vm: &VirtualMachine, _address: Relocatable) -> Result<usize, MemoryError> {
        Ok(8)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "String")]
pub struct UInt384(pub BigUint);

impl TryFrom<String> for UInt384 {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let hex_str = value.strip_prefix("0x").unwrap_or(&value);
        BigUint::parse_bytes(hex_str.as_bytes(), 16)
            .map(UInt384)
            .ok_or_else(|| format!("Invalid hex string: {}", value))
    }
}

impl UInt384 {
    pub fn to_limbs(&self) -> [Felt252; 4] {
        const LIMB_SIZE: u32 = 96;
        let limb_mask = (BigUint::from(1u128) << LIMB_SIZE) - BigUint::from(1u128);

        let d0: BigUint = &self.0 & &limb_mask;
        let d1: BigUint = (&self.0 >> 96) & &limb_mask;
        let d2: BigUint = (&self.0 >> 192) & &limb_mask;
        let d3: BigUint = (&self.0 >> 288) & &limb_mask;

        [
            Felt252::from_bytes_be_slice(&d0.to_bytes_be()),
            Felt252::from_bytes_be_slice(&d1.to_bytes_be()),
            Felt252::from_bytes_be_slice(&d2.to_bytes_be()),
            Felt252::from_bytes_be_slice(&d3.to_bytes_be()),
        ]
    }
}

impl CairoType for UInt384 {
    fn from_memory(vm: &VirtualMachine, address: Relocatable) -> Result<Self, MemoryError> {
        let d0 = BigUint::from_bytes_be(&vm.get_integer((address + 0)?)?.to_bytes_be());
        let d1 = BigUint::from_bytes_be(&vm.get_integer((address + 1)?)?.to_bytes_be());
        let d2 = BigUint::from_bytes_be(&vm.get_integer((address + 2)?)?.to_bytes_be());
        let d3 = BigUint::from_bytes_be(&vm.get_integer((address + 3)?)?.to_bytes_be());
        let bigint = d3 << 288 | d2 << 192 | d1 << 96 | d0;
        Ok(Self(bigint))
    }

    fn to_memory(&self, vm: &mut VirtualMachine, address: Relocatable) -> Result<Relocatable, MemoryError> {
        let limbs = self.to_limbs();

        vm.insert_value((address + 0)?, &limbs[0])?;
        vm.insert_value((address + 1)?, &limbs[1])?;
        vm.insert_value((address + 2)?, &limbs[2])?;
        vm.insert_value((address + 3)?, &limbs[3])?;

        Ok((address + 4)?)
    }

    fn n_fields(_vm: &VirtualMachine, _address: Relocatable) -> Result<usize, MemoryError> {
        Ok(4)
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Felt(pub Felt252);

impl CairoType for Felt {
    fn from_memory(vm: &VirtualMachine, address: Relocatable) -> Result<Self, MemoryError> {
        let value = vm.get_integer((address + 0)?)?;
        Ok(Self(*value))
    }

    fn to_memory(&self, vm: &mut VirtualMachine, address: Relocatable) -> Result<Relocatable, MemoryError> {
        vm.insert_value((address + 0)?, &self.0)?;
        Ok((address + 1)?)
    }

    fn n_fields(_vm: &VirtualMachine, _address: Relocatable) -> Result<usize, MemoryError> {
        Ok(1)
    }
}

#[derive(Debug, Deserialize)]
pub struct G1CircuitPoint{
    x: UInt384,
    y: UInt384,
}

impl CairoType for G1CircuitPoint {
    fn from_memory(vm: &VirtualMachine, address: Relocatable) -> Result<Self, MemoryError> {
        let x = UInt384::from_memory(vm, address)?;
        let y = UInt384::from_memory(vm, (address + 4)?)?;
        Ok(Self{x, y})
    }

    fn to_memory(&self, vm: &mut VirtualMachine, address: Relocatable) -> Result<Relocatable, MemoryError> {
        self.x.to_memory(vm, address)?;
        self.y.to_memory(vm, (address + 4)?)?;
        Ok((address + 8)?)
    }

    fn n_fields(_vm: &VirtualMachine, _address: Relocatable) -> Result<usize, MemoryError> {
        Ok(8)
    }
}

#[derive(Debug, Deserialize)]
pub struct G2CircuitPoint{
    x0: UInt384,
    x1: UInt384,
    y0: UInt384,
    y1: UInt384,
}

impl CairoType for G2CircuitPoint {
    fn from_memory(vm: &VirtualMachine, address: Relocatable) -> Result<Self, MemoryError> {
        let x0 = UInt384::from_memory(vm, address)?;
        let x1 = UInt384::from_memory(vm, (address + 4)?)?;
        let y0 = UInt384::from_memory(vm, (address + 8)?)?;
        let y1 = UInt384::from_memory(vm, (address + 12)?)?;
        Ok(Self{x0, x1, y0, y1})
    }

    fn to_memory(&self, vm: &mut VirtualMachine, address: Relocatable) -> Result<Relocatable, MemoryError> {
        self.x0.to_memory(vm, address)?;
        self.x1.to_memory(vm, (address + 4)?)?;
        self.y0.to_memory(vm, (address + 8)?)?;
        self.y1.to_memory(vm, (address + 12)?)?;
        Ok((address + 16)?)

    }

    fn n_fields(_vm: &VirtualMachine, _address: Relocatable) -> Result<usize, MemoryError> {
        Ok(16)
    }
}
