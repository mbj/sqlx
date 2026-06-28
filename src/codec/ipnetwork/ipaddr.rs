use std::net::IpAddr;

use ipnetwork::IpNetwork;

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueRef};

impl Type for IpAddr
where
    IpNetwork: Type,
{
    fn type_info() -> TypeInfo {
        IpNetwork::type_info()
    }

    fn compatible(ty: &TypeInfo) -> bool {
        IpNetwork::compatible(ty)
    }
}

impl HasArrayType for IpAddr {
    fn array_type_info() -> TypeInfo {
        <IpNetwork as HasArrayType>::array_type_info()
    }

    fn array_compatible(ty: &TypeInfo) -> bool {
        <IpNetwork as HasArrayType>::array_compatible(ty)
    }
}

impl<'db> Encode<'db> for IpAddr
where
    IpNetwork: Encode<'db>,
{
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        IpNetwork::from(*self).encode_by_ref(buf)
    }

    fn size_hint(&self) -> usize {
        IpNetwork::from(*self).size_hint()
    }
}

impl<'db> Decode<'db> for IpAddr
where
    IpNetwork: Decode<'db>,
{
    fn decode(value: ValueRef<'db>) -> Result<Self, BoxDynError> {
        let ipnetwork = IpNetwork::decode(value)?;

        if ipnetwork.is_ipv4() && ipnetwork.prefix() != 32
            || ipnetwork.is_ipv6() && ipnetwork.prefix() != 128
        {
            Err("lossy decode from inet/cidr")?
        }

        Ok(ipnetwork.ip())
    }
}
