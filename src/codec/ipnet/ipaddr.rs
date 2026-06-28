use std::net::IpAddr;

use ipnet::IpNet;

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueRef};

impl Type for IpAddr
where
    IpNet: Type,
{
    fn type_info() -> TypeInfo {
        IpNet::type_info()
    }

    fn compatible(ty: &TypeInfo) -> bool {
        IpNet::compatible(ty)
    }
}

impl HasArrayType for IpAddr {
    fn array_type_info() -> TypeInfo {
        <IpNet as HasArrayType>::array_type_info()
    }

    fn array_compatible(ty: &TypeInfo) -> bool {
        <IpNet as HasArrayType>::array_compatible(ty)
    }
}

impl<'db> Encode<'db> for IpAddr
where
    IpNet: Encode<'db>,
{
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        IpNet::from(*self).encode_by_ref(buf)
    }

    fn size_hint(&self) -> usize {
        IpNet::from(*self).size_hint()
    }
}

impl<'db> Decode<'db> for IpAddr
where
    IpNet: Decode<'db>,
{
    fn decode(value: ValueRef<'db>) -> Result<Self, BoxDynError> {
        let ipnet = IpNet::decode(value)?;

        if matches!(ipnet, IpNet::V4(net) if net.prefix_len() != 32)
            || matches!(ipnet, IpNet::V6(net) if net.prefix_len() != 128)
        {
            Err("lossy decode from inet/cidr")?
        }

        Ok(ipnet.addr())
    }
}
