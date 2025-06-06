#!/usr/bin/perl -w
use strict;

# Create a layout file for `lpx_ctl`that demonstrates lots of colours
# and basic MIDI output

my @col = qw |127   0   0  red
  0 127   0  Lime
  0   0 127  Royal_Blue
127 127   0  Orange
  0 127 127  Teal
127   0 127  Purple
 64  64  64  Grey
 85   0  85  Electric_Purple
127  64   0  Burnt_Orange
  0  64 127  Deep_Sea
127   0  64  Red_violet
 32 127   0  Yellow
  0 127  64  Mint_Green
 64   0 127  Indigo
127   0  32  Violet_Red
 96 127   0  Orange_Yellow
  0  96 127  Deep_Teal
127   0  96  Dark_Magenta
 64 127   0  Gold
  0  32 127  Ocean_Blue
127  64  64  Salmon
 32  64   0  Olive
  0 127  32  Bright_Green
 32   0 127  Midnight_Blue
127  96   0  Dark_Orange
  0  64  64  Teal_grey
 96   0 127  Grape
127  32   0  Brick_Red
 32  96  64  Frost
  0 127  96  Turquoise
 64  32   0  Umber
 32   0  96  Gothic
127  96  96  Pinkish_Tan
 64  64   0  Olive_Drab
  0  64  32  Emerald_Green
 32   0  64  Amethyst
127 127  96  Pale_Yellow
 64  96  32  Sage
  0  32  64  Lapis_Blue
 32   0  32  Plum
 96 127 127  Aquamarine
 64  32  64  Smoky_Pink
 42 127  85  Jade_Green
  0  96  32  Forest_Green
 32  64  96  Steel_Blue
 96   0  32  Maroon
 64 127  64  Spring_green
  0  32  96  Ultramarine
 32  64  32  Fern
 96  32   0  Rust
 64  96  96  Tea_Green
  0   0  64  Navy_Blue
 32  32  64  Cornflower
 96  64  64  Cinnamon_pink
  0   0  32  Sapphire_Blue
 32   0   0  Dark_Brown
 64  32  32  Terracotta
 32  32   0  Hunter_Green
127  64 127  Magenta_Pink
 96 127  96  Pale_Olive
 64  64 127  Cornflower_Blue
 96  96 127  Periwinkle
 32  64  64  Greyish_Teal
 64   0  32  Burgundy
|;

my @colours =  ();

my $c = 0;
my $colour = "";
for (my $i = 0; $i < scalar(@col); $i++){
    if($c < 3){
	$colour .= "$col[$i] ";
    }else{
	my $name = $col[$i];
	chomp $name;
	my ($red, $green, $blue) = split(/\s/, $colour);
	my $r_a = ($red + 64) % 127;
	my $g_a = ($green + 64) % 127;
	my $b_a = ($blue + 64) % 127;
	
	push(@colours, "$red $green $blue $name");
	push(@colours, "$r_a $g_a $b_a ".$name.'_active');
	$colour = '';
    }
    $c++;
    $c %= 4;
}
print join("\n", @colours)."\n\n";
my $row =1;
my $col= 1;
my $s = 1;
my $midi = 33;
for(my $i = 0; $i < scalar(@colours); $i += 2){
    my $l = $colours[$i];
    my $l_a = $colours[$i+1];
    
    my @l = split(/ /, $l);
    my @la = split(/ /, $l_a);
    my $l1 = $l[3];
    my $l1a = $la[3];
    print "s$s $l1 $l1a $midi [$col$row]\n";
    $s++;
    $row++;
    $midi++;
    if($row == 9){
	$row = 1;
	$col++;
	$col == 9 and $col = 1;
    }
}


