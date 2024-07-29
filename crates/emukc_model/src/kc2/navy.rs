/*
let list = vec![
299, // Scamp
433, // Saratoga
440, // Iowa
544, // Gambier Bay
549, // Intrepid
561, // Samuel B. Roberts
562, // Johnston
595, // Houston
596, // Fletcher
597, // Atlanta
598, // Honolulu
601, // Colorado
602, // South Dakota
603, // Hornet
615, // Helena
654, // Washington
655, // Northampton
891, // Salmon
892, // Drum
896, // Brooklyn
913, // Maryland
923, // Tuscaloosa
924, // Nevada
925, // Langley
931, // Ranger
933, // Massachusetts
941, // Heywood L. Edwards
];

let ids: Vec<i64> = list
   .iter()
   .filter_map(|id| {
	   let mst = codex.find_ship_mst(*id).unwrap();
	   debug!("looking for all submodels of: {} {}", id, mst.api_name);
	   codex.ship_and_after(*id).ok()
   })
   .flat_map(|s| {
	   s.iter().for_each(|id| {
		   let mst = codex.find_ship_mst(*id).unwrap();
		   info!("\tall subships: {}, {}", mst.api_id, mst.api_name);
	   });
	   s.into_iter()
   })
   .collect();

info!("USN: {:?}", ids);

ids.iter().for_each(|id| {
   let mst = codex.find_ship_mst(*id).unwrap();
   info!("ship: {}, {}", mst.api_id, mst.api_name);
});
*/

/// United States Navy
pub const USN: [i64; 61] = [
	299, 715, 433, 438, 545, 550, 440, 360, 544, 396, 707, 549, 397, 561, 681, 920, 562, 689, 595,
	600, 596, 692, 628, 629, 597, 696, 598, 711, 601, 1496, 602, 697, 603, 704, 615, 620, 654, 659,
	655, 660, 891, 897, 892, 732, 896, 722, 913, 918, 923, 928, 924, 929, 936, 925, 930, 931, 723,
	933, 938, 941, 726,
];

/*
let list = [
439, // Warspite
514, // Sheffield
515, // Ark Royal
519, // Jervis
520, // Janus
571, // Nelson
572, // Rodney
885, // Victorious
901, // Javelin
];
 */

/// Royal Navy
pub const RN: [i64; 18] =
	[439, 364, 514, 705, 515, 393, 519, 394, 520, 893, 571, 576, 572, 577, 885, 713, 901, 906];

/*
613, // Perth
 */
/// Royal Australian Navy
pub const RAN: [i64; 2] = [613, 618];

/*
604, // De Ruyter
 */
/// Royal Netherlands Navy
pub const RNN: [i64; 2] = [604, 609];
