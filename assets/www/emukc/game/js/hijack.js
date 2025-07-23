var VERSION = "{{ version }}";
// kcs_const.js
var originWithSlash = document.location.origin + "/";
var ConstServerInfo = {};
ConstServerInfo.Gadget = originWithSlash;
ConstServerInfo.OSAPI = document.location.host + "&dmmuser_id={{uid}}";
ConstServerInfo.NETGAME = originWithSlash;

console.log(ConstServerInfo);

for (var i = 1; i <= 20; i++) {
	ConstServerInfo["World_" + i] = originWithSlash;
}

var ConstURLInfo = {};
ConstURLInfo.GetUserWorldURL = originWithSlash + "kcsapi/api_world/get_id/";
ConstURLInfo.ConnectionCheckURL = ConstServerInfo.Gadget + "index.html";
ConstURLInfo.LoginURL = "/kcsapi/api_auth_member/dmmlogin/";

// Connection 情報
var ConnectionInfo = {};
ConnectionInfo.Interval_Min = 10; // change this to test keep-alive

// Maintenance 情報
var MaintenanceInfo = {};
MaintenanceInfo.IsDoing = 0;
MaintenanceInfo.IsEmergency = 0;
MaintenanceInfo.StartDateTime = Date.parse("2077/05/01 00:00:00");
MaintenanceInfo.EndDateTime = Date.parse("2077/05/01 00:59:59");

// Entrance 情報
var EntranceInfo = {};
EntranceInfo.Groups = 10;
EntranceInfo.Interval_Min = 1;
EntranceInfo.UidIndex = 0;
// Entrance 情報（ワールド別）
EntranceInfo.NewUser = 2; // allow new user
for (var i = 1; i <= 20; i++) {
	EntranceInfo["World_" + i + "_User"] = 2;
}

// kcs_options.js
function kcsOptions_Save(options) {
	var sKey = "kcs_options";
	var sValue = options;
	var vEnd = 2592000;
	var sPath = "/";
	var sDomain = "localhost";
	var bSecure = false;
	docCookies.setItem(sKey, sValue, vEnd, sPath, sDomain, bSecure);
}

// dmm.js
gadgets.error = console.error;
gadgets.warn = console.warn;
gadgets.log = console.log;
gadgets.info = console.info;
gadgets.debug = console.debug;

var REPO_URL = "https://github.com/master-g/emukc.git";
var type = "DEBUG";

if (navigator.userAgent.toLowerCase().indexOf("chrome") > -1) {
	const args = [
		`\n %c %c %c EMUKC ${VERSION} - ✰ ${type} ✰  %c  %c  ${REPO_URL}  %c %c ♥%c♥%c♥ \n\n`,
		"background: #e15e04; padding:5px 0;",
		"background: #e15e04; padding:5px 0;",
		"color: #f8c104; background: #cc0404; padding:5px 0;",
		"background: #d43004; padding:5px 0;",
		"background: #e15e04; padding:5px 0;",
		"background: #d43004; padding:5px 0;",
		"color: #f8c104; background: #cc0404; padding:5px 0;",
		"color: #f8c104; background: #cc0404; padding:5px 0;",
		"color: #f8c104; background: #cc0404; padding:5px 0;",
	];

	console.log(...args);
} else {
	console.warn(`EmuKC ${VERSION} - ${type} - ${REPO_URL}`);
}
