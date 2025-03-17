use emukc_model::kc2::start2::ApiManifest;

use crate::make_list::CacheList;

pub(super) fn make(mst: &ApiManifest, list: &mut CacheList) {
	for item in mst.api_mst_payitem.iter() {
		list.add_unversioned(format!("kcs/images/purchase_items/{}.jpg", item.api_id));
	}
}
